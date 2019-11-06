use tonic::{transport::Server, Request, Response, Status};

const SERVER_PORT: i32 = 50051;

pub mod rust_chat {
  tonic::include_proto!("rustchat");
}

use rust_chat::{
  server::{ChatRoom, ChatRoomServer},
  LoginRequest, LoginReply,
};

pub struct MyChatRoom {}

#[tonic::async_trait]
impl ChatRoom for MyChatRoom {
  async fn login(
    &self,
    request: Request<LoginRequest>
  ) -> Result<Response<LoginReply>, Status> {
    let reply = rust_chat::LoginReply {
      ok: false.into()
    };

    Ok(Response::new(reply))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr = format!("[::1]:{}", SERVER_PORT).parse()?;
  let chatroom = MyChatRoom {};

  println!("Server listening in on port {}", SERVER_PORT);

  Server::builder()
    .add_service(ChatRoomServer::new(chatroom))
    .serve(addr)
    .await?;

  Ok(())
}