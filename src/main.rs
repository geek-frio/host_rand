use clap::Parser;
use rand::prelude::*;
use regex::Regex;
use std::{collections::HashSet, fs::OpenOptions, io::Read, ops::Index, sync::mpsc::Receiver};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    // 每10分钟change一次绑定ip
    #[clap(short, long, default_value = "10")]
    rate: usize,
    #[clap(short, long)]
    host: String,
}

fn comment_all(buf: &mut Vec<String>) {
    buf.iter_mut()
        .filter(|a| !check_comment_prefix(*a))
        .for_each(|a| {
            a.insert_str(0, "#");
        });
}

fn check_comment_prefix(s: &str) -> bool {
    let mut chs = s.chars();
    while let Some(c) = chs.next() {
        match c {
            ' ' => continue,
            '#' => return true,
            _ => return false,
        }
    }
    false
}

fn choose_one(buf: &mut Vec<String>, black_list: &mut HashSet<String>) {
    let mut thread_rng = thread_rng();
    loop {
        let idx = thread_rng.gen_range(0..buf.len());
        let s = &buf[idx];
        if black_list.contains(s) {
            continue;
        } else {
            uncomment_prefix(buf, idx);
        }
    }
}

fn uncomment_prefix(buf: &mut Vec<String>, idx: usize) {
    let s = buf.get_mut(idx).expect("get comment host failed");
    if check_comment_prefix(s) {
        let chs = s.chars().into_iter();
        let mut flag = false;
        let s = chs
            .filter(|a| {
                if !flag {
                    if *a == '#' {
                        flag = true;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .collect::<String>();
        buf[idx] = s;
    }
}

fn re_select(buf: &mut Vec<String>, black_list: &mut HashSet<String>) {
    comment_all(buf);
    choose_one(buf, black_list);
}

fn group_host_file<'a>(host: &'a str, s: &'a String) -> (Vec<String>, Vec<String>) {
    let lines = host.split('\n');
    let mut hosts: Vec<String> = Vec::new();
    let not_hosts: Vec<String> = lines
        .filter(|a| {
            if a.contains(&host) {
                hosts.push(a.to_string());
                return false;
            } else {
                true
            }
        })
        .map(|a| a.to_string())
        .collect();
    (not_hosts, hosts)
}

fn select_on_event(not_hosts: Vec<&str>, hosts: Vec<&str>) {
    todo!();
}

fn flush(not_hosts: &Vec<String>, hosts: &Vec<String>) {
    todo!()
}

fn read_hosts_content() -> String {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/etc/hosts")
        .expect("编辑 /etc/hosts文件 失败,没有权限！");
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .expect("/etc/hosts 文件内容读取失败！");
    buf
}

enum Event {
    Default,
}

fn main() {
    let args = Args::parse();
    let (send, recv) = std::sync::mpsc::channel::<Event>();

    let _ = send.send(Event::Default);
    loop {
        let event = recv.recv().unwrap();
        match event {
            Event::Default => {
                let content = read_hosts_content();
                let (not_hosts, mut hosts) = group_host_file(args.host.as_str(), &content);
                let mut black_list = HashSet::new();
                re_select(&mut hosts, &mut black_list);
                flush(&not_hosts, &hosts);
            }
        }
    }
}
