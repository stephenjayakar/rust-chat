#[macro_use] extern crate text_io;

const SERVER_PORT: i32 = 50051;

pub mod rust_chat {
  tonic::include_proto!("rustchat");
}

use rust_chat::{
  client::ChatRoomClient,
  LoginRequest,
  SendMessageRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Enter a username...");
  let username: String = read!();
  let mut client = ChatRoomClient::connect(format!("http://[::1]:{}", SERVER_PORT)).await?;

  let request = tonic::Request::new(LoginRequest {
        username: username.clone().into(),
  });

  let response = client.login(request).await?;

  let mut looping = response.into_inner().ok;
  println!("successful login as username {}!", username);
  println!("write messages followed with enter");
  while looping {
    let message: String = read!("{}\n");
    let request = tonic::Request::new(SendMessageRequest {
      username: username.clone().into(),
      message: message.clone().into(),
    });
    let response = client.send_message(request).await?;
    looping = response.into_inner().ok;
  }
  Ok(())
}