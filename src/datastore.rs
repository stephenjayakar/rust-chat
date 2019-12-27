// Abstraction for data store
/*  right now is in-memory, will probably move to some type
    of SQL database.  also, wanted to move mutex logic out
    of main server
*/
// TODO: figure out why we need this?
#![allow(dead_code)]
use std::{collections::HashSet, vec::Vec};
use tokio::sync::Mutex;

pub struct DataStore {
  users: Mutex<HashSet<String>>,
  messages: Mutex<Vec<String>>,
}

impl DataStore {
  pub fn new() -> DataStore {
    DataStore {
      users: Mutex::new(HashSet::new()),
      messages: Mutex::new(Vec::new())
    }
  }
  // returns false if user already exists
  pub async fn create_user(&self, username: &str) -> bool {
    let mut users = self.users.lock().await;
    users.insert(username.to_string())
  }

  pub async fn user_exists(&self, username: &str) -> bool {
    let users = self.users.lock().await;
    users.contains(username)
  }

  pub async fn add_message(&self, message: String) {
    let mut messages = self.messages.lock().await;
    messages.push(message.clone());
  }

  // TODO: add some type of cursor to not go through all messages
  // TODO: change this to events to support login messages
  pub async fn get_messages(&self, cursor: usize) -> Vec<String> {
    let messages = self.messages.lock().await;
    messages[cursor..].to_owned()
  }
}
