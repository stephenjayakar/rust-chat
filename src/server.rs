use std::convert::TryInto;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::timer::delay;
use tonic::{transport::Server, Request, Response, Status};

mod datastore;
use datastore::DataStore;

const SERVER_PORT: i32 = 50051;

pub mod rust_chat {
  tonic::include_proto!("rustchat");
}

use rust_chat::{
  server::{ChatRoom, ChatRoomServer},
  GetMessageStreamReply, GetMessageStreamRequest, LoginReply, LoginRequest, SendMessageReply,
  SendMessageRequest, TestStreamReply, TestStreamRequest,
};

pub struct MyChatRoom {
  data_store: DataStore,
  subscriptions: Mutex<Vec<mpsc::UnboundedSender<Result<GetMessageStreamReply, Status>>>>,
}

impl MyChatRoom {
  async fn _add_message(&self, msg: String) {
    self.data_store.add_message(msg.clone()).await;
    let mut subscriptions = self.subscriptions.lock().await;
    let mut indexes_to_remove = Vec::new();
    for (i, tx) in subscriptions.iter_mut().enumerate() {
      let reply = GetMessageStreamReply {
        message: msg.clone(),
      };
      if tx.try_send(Ok(reply)).is_err() {
        indexes_to_remove.push(i);
        println!("DEBUG: removing a client!");
      }
    }
    for i in indexes_to_remove {
      subscriptions.swap_remove(i);
    }
  }
}

#[tonic::async_trait]
impl ChatRoom for MyChatRoom {
  async fn login(&self, request: Request<LoginRequest>) -> Result<Response<LoginReply>, Status> {
    let username = request.into_inner().username;
    let success = self.data_store.create_user(&username).await;
    if success {
      let msg = format!(
        "{} logged on!  Sending current messages in chatroom...",
        username
      );
      self._add_message(msg).await;
    }
    let reply = rust_chat::LoginReply { ok: success };
    Ok(Response::new(reply))
  }

  async fn send_message(
    &self,
    request: Request<SendMessageRequest>,
  ) -> Result<Response<SendMessageReply>, Status> {
    let request = request.into_inner();
    let username = request.username;
    let message = request.message;
    let msg = format!("{}: {}", username, message);
    if !self.data_store.user_exists(&username).await {
      let reply = SendMessageReply { ok: false };
      return Ok(Response::new(reply));
    }
    self._add_message(msg.clone()).await;

    let reply = SendMessageReply { ok: true };
    Ok(Response::new(reply))
  }

  type TestStreamStream = mpsc::Receiver<Result<TestStreamReply, Status>>;

  async fn test_stream(
    &self,
    request: Request<TestStreamRequest>,
  ) -> Result<Response<Self::TestStreamStream>, Status> {
    let message = request.into_inner().message;
    println!("Received streaming request with message {}", message);

    let (mut tx, rx) = mpsc::channel::<Result<TestStreamReply, Status>>(4);

    tokio::spawn(async move {
      for character in message.chars() {
        println!("sending {}", character);
        let character = character.to_string();
        let reply = TestStreamReply { character };
        if tx.send(Ok(reply)).await.is_err() {
          println!("client dropped!");
          return;
        }

        let when = tokio::clock::now() + Duration::from_secs(1);
        delay(when).await;
      }
      println!("done sending!")
    });
    Ok(Response::new(rx))
  }

  type GetMessageStreamStream = mpsc::UnboundedReceiver<Result<GetMessageStreamReply, Status>>;

  async fn get_message_stream(
    &self,
    request: Request<GetMessageStreamRequest>,
  ) -> Result<Response<Self::GetMessageStreamStream>, Status> {
    let cursor = request.into_inner().cursor;

    let (mut tx, rx) = mpsc::unbounded_channel::<Result<GetMessageStreamReply, Status>>();
    let messages = self
      .data_store
      .get_messages(cursor.try_into().unwrap())
      .await;
    for message in messages {
      println!("sending message {}", message);
      let reply = GetMessageStreamReply { message: message };
      tx.try_send(Ok(reply)).unwrap();
    }
    // add tx to list
    let mut subscriptions = self.subscriptions.lock().await;
    subscriptions.push(tx);
    println!("done sending initial batch!");
    Ok(Response::new(rx))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr = format!("[::1]:{}", SERVER_PORT).parse()?;
  let chatroom = MyChatRoom {
    data_store: DataStore::new(),
    subscriptions: Mutex::new(Vec::new()),
  };

  println!("Server listening in on port {}", SERVER_PORT);

  Server::builder()
    .add_service(ChatRoomServer::new(chatroom))
    .serve(addr)
    .await?;

  Ok(())
}
