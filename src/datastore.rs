// Abstraction for data store
/*  right now is in-memory, will probably move to some type
    of SQL database.  also, wanted to move mutex logic out
    of main server
*/
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
  pub async fn create_user(&self, username: String) -> bool {
    let mut users = self.users.lock().await;
    users.insert(username.clone())
  }

  pub async fn user_exists(&self, username: String) -> bool {
    let mut users = self.users.lock().await;
    users.contains(&username)
  }

  pub async fn add_message(&self, message: String) {
    let mut messages = self.messages.lock().await;
    messages.push(message.clone());
  }

  // TODO: add some type of cursor to not go through all messages
  // TODO: change this to events to support login messages
  pub async fn get_messages(&self) -> Vec<String> {
    let messages = self.messages.lock().await;
    messages.clone()
  }
}