#[macro_use]
extern crate text_io;
extern crate termion;

use termion::{clear, cursor};
use tonic::transport::Channel;
use tonic::Request;

const SERVER_PORT: i32 = 50051;

pub mod rust_chat {
  tonic::include_proto!("rustchat");
}

mod datastore;

use rust_chat::{
  client::ChatRoomClient, GetMessageStreamRequest, LoginRequest, SendMessageRequest,
};

async fn print_messages(
  mut client: ChatRoomClient<Channel>,
) -> Result<(), Box<dyn std::error::Error>> {
  let cursor = 0;
  let request = Request::new(GetMessageStreamRequest { cursor: cursor });
  let mut stream = client.get_message_stream(request).await?.into_inner();
  while let Some(reply) = stream.message().await? {
    println!("{}", reply.message);
  }
  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Enter a username...");
  let username: String = read!();
  let mut client = ChatRoomClient::connect(format!("http://[::1]:{}", SERVER_PORT)).await?;

  let request = Request::new(LoginRequest {
    username: username.clone(),
  });

  let response = client.login(request).await?;

  let mut looping = response.into_inner().ok;
  println!("successful login as username {}!", username);
  println!("write messages followed with enter");

  let message_client = client.clone();
  tokio::spawn(async move {
    print_messages(message_client).await.unwrap();
  });

  println!("write your messages, followed by enter");
  while looping {
    let message: String = read!("{}\n");

    print!(
      "{clear}{goto}",
      clear = clear::CurrentLine,
      goto = cursor::Up(1)
    );

    let message = message.trim();
    if message.is_empty() {
      continue;
    }
    let request = Request::new(SendMessageRequest {
      username: username.clone(),
      message: message.into(),
    });
    let response = client.send_message(request).await?;
    looping = response.into_inner().ok;
  }
  Ok(())
}
