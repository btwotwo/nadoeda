use async_trait::async_trait;
use tokio::{sync::mpsc, task::JoinHandle};

#[async_trait]
pub trait Actor: Sized {
    type Message: Send + 'static;
    type State: Send + 'static;
    type InitArgs: Send;

    fn handle_message(
        msg: Self::Message,
        state: Self::State,
        context: &ActorContext<Self>,
    ) -> anyhow::Result<Self::State>;
    
    async fn init_state(args: Self::InitArgs) -> anyhow::Result<Self::State>;
}

pub struct ActorContext<TActor: Actor> {
    sender: mpsc::UnboundedSender<TActor::Message>,
}

#[derive(Clone)]
pub struct ActorReference<TActor: Actor>(mpsc::UnboundedSender<TActor::Message>);

impl<TActor: Actor> ActorReference<TActor> {
    pub fn send_message(&self, msg: TActor::Message) {
        self.0.send(msg).unwrap()
    }    
}

pub struct ActorHandle<TActor: Actor> {
    task: JoinHandle<()>,
    reference: ActorReference<TActor>
}

impl<TActor: Actor> ActorHandle<TActor> {
    pub fn actor_reference(&self) -> &ActorReference<TActor> {
        &self.reference
    }
}

pub async fn start<TActor: Actor>(args: TActor::InitArgs) -> ActorHandle<TActor> {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let sender_clone = sender.clone();
    let initial_state = TActor::init_state(args).await.unwrap();

    let task = tokio::spawn(async move {
        let mut state = initial_state;
        let context = ActorContext {
            sender: sender_clone,
        };
        while let Some(msg) = receiver.recv().await {
            state = TActor::handle_message(msg, state, &context).unwrap();
        }
    });

    ActorHandle { task, sender }
}
