use async_trait::async_trait;
use tokio::{sync::mpsc, task::JoinHandle};

#[async_trait]
pub trait Actor: Sized {
    type Message: Send;
    type State: Send;
    type InitArgs: Send;

    fn handle_message(
        msg: Self::Message,
        state: Self::State,
        context: ActorContext<Self>,
    ) -> anyhow::Result<Self::State>;
    async fn init_state(args: Self::InitArgs) -> anyhow::Result<Self::State>;
}

pub struct ActorContext<TActor: Actor> {
    sender: mpsc::UnboundedSender<TActor::Message>,
}

pub struct ActorHandle<TActor: Actor> {
    task: JoinHandle<()>,
    sender: mpsc::UnboundedSender<TActor::Message>,
}

pub fn start<TActor: Actor>(args: TActor::InitArgs) -> ActorHandle<TActor> {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let sender_clone = sender.clone();
    let task = tokio::spawn(async move {
        let mut state = TActor::init_state(args).await.unwrap();
        let context = ActorContext {
            sender: sender_clone,
        };
        while let Some(msg) = receiver.recv().await {
            state = TActor::handle_message(msg, state, context).unwrap();
        }
    });

    ActorHandle { task, sender }
}
