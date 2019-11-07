use tonic::{transport::Server, Request, Response, Status};
use std::collections::HashSet;
use std::vec::Vec;
use tokio::sync::Mutex;

const SERVER_PORT: i32 = 50051;

pub mod rust_chat {
  tonic::include_proto!("rustchat");
}

use rust_chat::{
  server::{ChatRoom, ChatRoomServer},
  LoginRequest, LoginReply,
  SendMessageRequest, SendMessageReply,
};

pub struct MyChatRoom {
  users: Mutex::<HashSet<String>>,
  messages: Mutex::<Vec<MessageEntry>>,
}

struct MessageEntry {
  username: String,
  message: String,
}

#[tonic::async_trait]
impl ChatRoom for MyChatRoom {
  async fn login(
    &self,
    request: Request<LoginRequest>
  ) -> Result<Response<LoginReply>, Status> {
    let mut users = self.users.lock().await;
    let username = request.into_inner().username;
    let success = users.insert(username.clone());
    if success {
      println!("User {} logged on!", username);
    }

    let reply = rust_chat::LoginReply {
      ok: success,
    };
    Ok(Response::new(reply))
  }

  async fn send_message(
    &self,
    request: Request<SendMessageRequest>
  ) -> Result<Response<SendMessageReply>, Status> {
    let users = self.users.lock().await;
    let mut messages = self.messages.lock().await;
    let request = request.into_inner();
    let username = request.username;
    match users.contains(&username) {
      true => {
        let message = request.message;
        messages.push(MessageEntry {
          username: username.clone(),
          message: message.clone(),
        });
        println!("{}: {}", username, message);
        let reply = rust_chat::SendMessageReply {
          ok: true
        };
        Ok(Response::new(reply))
      }
      false => Ok(Response::new(rust_chat::SendMessageReply { ok: false }))
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr = format!("[::1]:{}", SERVER_PORT).parse()?;
  let chatroom = MyChatRoom {
    users: Mutex::new(HashSet::new()),
    messages: Mutex::new(Vec::new()),
  };

  println!("Server listening in on port {}", SERVER_PORT);

  Server::builder()
    .add_service(ChatRoomServer::new(chatroom))
    .serve(addr)
    .await?;

  Ok(())
}