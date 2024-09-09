#![allow(dead_code)]

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub servers: Servers,
}

#[derive(Deserialize)]
pub struct Servers {
    pub control: Box<[Box<str>]>,
    pub worker: Box<[Box<str>]>,
    pub vip: Option<Box<str>>,
}
