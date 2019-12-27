use std::convert::TryInto;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::timer::delay;
use tonic::{transport::Server, Request, Response, Status};

mod datastore;
use datastore::DataStore;

const SERVER_PORT: i32 = 50051;
// How often to check if clients are logged in in seconds
const HEARTBEAT_RATE: u64 = 1;
const EMPTY_MESSAGE: &str = "";

pub mod rust_chat {
  tonic::include_proto!("rustchat");
}

use rust_chat::{
  server::{ChatRoom, ChatRoomServer},
  GetMessageStreamReply, GetMessageStreamRequest, LoginReply, LoginRequest, SendMessageReply,
  SendMessageRequest,
};

pub struct MyChatRoom {
  data_store: DataStore,
  // All open senders to clients
  subscriptions: Mutex<
    Vec<(
      String,
      mpsc::UnboundedSender<Result<GetMessageStreamReply, Status>>,
    )>,
  >,
}

impl MyChatRoom {
  async fn add_message(&self, msg: String) {
    self.data_store.add_message(msg.clone()).await;
    let mut subscriptions = self.subscriptions.lock().await;
    self.add_message_with_lock(msg, &mut subscriptions).await;
  }

  async fn add_message_with_lock(
    &self,
    msg: String,
    subscriptions: &mut tokio::sync::MutexGuard<
      '_,
      Vec<(
        String,
        mpsc::UnboundedSender<Result<GetMessageStreamReply, Status>>,
      )>,
    >,
  ) {
    for (_, tx) in subscriptions.iter_mut() {
      let reply = GetMessageStreamReply {
        message: msg.clone(),
      };
      tx.try_send(Ok(reply));
    }
  }
}

#[tonic::async_trait]
impl ChatRoom for Arc<MyChatRoom> {
  async fn login(&self, request: Request<LoginRequest>) -> Result<Response<LoginReply>, Status> {
    let username = request.into_inner().username;
    let success = self.data_store.create_user(&username).await;
    if success {
      let msg = format!("{} logged on!", username);
      self.add_message(msg).await;
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
    self.add_message(msg.clone()).await;

    let reply = SendMessageReply { ok: true };
    Ok(Response::new(reply))
  }

  type GetMessageStreamStream = mpsc::UnboundedReceiver<Result<GetMessageStreamReply, Status>>;

  async fn get_message_stream(
    &self,
    request: Request<GetMessageStreamRequest>,
  ) -> Result<Response<Self::GetMessageStreamStream>, Status> {
    let request = request.into_inner();
    let username = request.username;
    let cursor = request.cursor;

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
    let mut subscriptions = self.subscriptions.lock().await;
    subscriptions.push((username.clone(), tx));
    println!("done sending initial batch!");
    Ok(Response::new(rx))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr = format!("[::1]:{}", SERVER_PORT).parse()?;
  let chatroom = Arc::new(MyChatRoom {
    data_store: DataStore::new(),
    subscriptions: Mutex::new(Vec::new()),
  });

  let cloned_ref = chatroom.clone();
  // heartbeat loop
  tokio::spawn(async move {
    loop {
      let when = Instant::now() + Duration::new(HEARTBEAT_RATE, 0);
      delay(when).await;

      let mut subscriptions = cloned_ref.subscriptions.lock().await;
      let mut indexes_to_remove = Vec::new();
      for (i, (_, tx)) in subscriptions.iter_mut().enumerate() {
        let reply = GetMessageStreamReply {
          message: String::from(EMPTY_MESSAGE),
        };
        if tx.try_send(Ok(reply)).is_err() {
          indexes_to_remove.push(i);
        }
      }
      for i in indexes_to_remove {
        let username = subscriptions[i].0.clone();
        subscriptions.swap_remove(i);
        let msg = format!("{} logged out!", username);
        cloned_ref
          .add_message_with_lock(msg, &mut subscriptions)
          .await;
      }
    }
  });

  println!("Server listening in on port {}", SERVER_PORT);

  Server::builder()
    .add_service(ChatRoomServer::new(chatroom))
    .serve(addr)
    .await?;

  Ok(())
}
