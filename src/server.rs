use tonic::{transport::Server, Request, Response, Status};

mod datastore;
use datastore::DataStore;

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
  data_store: DataStore,
}

#[tonic::async_trait]
impl ChatRoom for MyChatRoom {
  async fn login(
    &self,
    request: Request<LoginRequest>
  ) -> Result<Response<LoginReply>, Status> {
    let username = request.into_inner().username;
    let success = self.data_store.create_user(username).await;
    if success {
      self.data_store.add_message(
        format!("{} logged on!", username)
      ).await;
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
    let request = request.into_inner();
    let username = request.username;
    let message = request.message;
    let user_exists = self.data_store.user_exists(username).await;
    if user_exists {
      self.data_store.add_message(format!("{}: {}", username, message)).await;
    }
    let reply = rust_chat::SendMessageReply {
      ok: user_exists,
    };
    Ok(Response::new(reply))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr = format!("[::1]:{}", SERVER_PORT).parse()?;
  let chatroom = MyChatRoom {
    data_store: DataStore::new(),
  };

  println!("Server listening in on port {}", SERVER_PORT);

  Server::builder()
    .add_service(ChatRoomServer::new(chatroom))
    .serve(addr)
    .await?;

  Ok(())
}