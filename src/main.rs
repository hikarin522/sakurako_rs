
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate serde;
extern crate serde_json;
extern crate slack;
extern crate docomo_api;

use std::fs::File;
use std::path::Path;
use std::error::Error;
use std::collections::HashMap;
use slack::{RtmClient, EventHandler};
use slack::Event;
use slack::Message;
use slack::Team;
use serde::de::Deserialize;
use serde_json as json;
use docomo_api::chat_dialogue as docomo;

#[derive(Deserialize, Debug)]
struct ID {
    slack: String,
    docomo: String,
}

fn get_json<P, T>(path: P) -> Result<T, Box<Error>>
    where P: AsRef<Path>,
          T: Deserialize
{
    let f = try!(File::open(path));
    let json = try!(json::from_reader(f));
    Ok(json)
}

#[derive(Debug)]
struct Handle {
    chat: docomo::Chat,
    name: String,
    team: String,
    users: HashMap<String, slack::User>,
}

impl Handle {
    fn new(id: &str) -> Handle {
        Handle {
            chat: docomo::Chat::new(id, docomo::Type::Sakurako),
            users: HashMap::new(),
            name: "".to_string(),
            team: "".to_string(),
        }
    }

    fn get_name(&self, id: &str) -> Option<String> {
        self.users.get(id).map(|ref u| u.name.clone())
    }

    fn get_users(&mut self, cli: &RtmClient) {
        let mut v = cli.get_users();
        self.users.clear();
        while let Some(u) = v.pop() {
            self.users.insert(u.id.clone(), u);
        }
    }

    fn reply(&mut self, msg: &str, name: &str) -> Result<docomo::Response, Box<Error>> {
        let mut req = docomo::Request::new(msg, &self.chat);
        let name = try!(self.get_name(name)
            .and_then(|x| {
                if x == self.name {
                    None
                } else {
                    Some(x)
                }
            })
            .ok_or("error"));
        println!("user name: {}", name);
        req.nickname = Some(name);
        self.chat.request(&req)
    }

    fn on_message(&mut self, cli: &mut RtmClient, msg: &Message) -> Result<(), Box<Error>> {
        match *msg {
            Message::Standard { channel: Some(ref ch),
                                user: Some(ref name),
                                text: Some(ref msg),
                                .. } => {
                let res = try!(self.reply(msg, name));
                println!("{:?}", res);
                try!(cli.send_message(ch, &res.utt));
            }
            _ => {}
        }
        Ok(())
    }
}

impl EventHandler for Handle {
    fn on_event(&mut self, cli: &mut RtmClient, ev: Result<&Event, slack::Error>, _: &str) {
        println!("on_event: {:?}", ev);
        if let Ok(e) = ev {
            match *e {
                Event::Message(ref msg) => {
                    let e = self.on_message(cli, msg);
                    println!("{:?}", e);
                }
                Event::UserChange { .. } |
                Event::TeamJoin { .. } => {
                    self.get_users(cli);
                }
                _ => {}
            }
        }
    }

    fn on_ping(&mut self, _: &mut RtmClient) {
        println!("on_ping");
    }

    fn on_close(&mut self, _: &mut RtmClient) {
        println!("on_close");
    }

    fn on_connect(&mut self, cli: &mut RtmClient) {
        println!("on_connect");
        self.name = cli.get_name()
            .unwrap_or("".to_string());
        self.team = cli.get_team()
            .map(|Team { name, .. }| name)
            .unwrap_or("".to_string());
        println!("bot name: {}", self.name);
        println!("team: {}", self.team);

        self.get_users(cli);
    }
}

fn main() {
    println!("Hello, world!");
    let id: ID = get_json("key.json").unwrap();
    println!("{:?}", id);

    let mut cli = RtmClient::new(&id.slack);
    let mut handle = Handle::new(&id.docomo);

    let e = cli.login_and_run(&mut handle);

    println!("e: {:?}", e);
}
