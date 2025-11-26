use std::process::Output;

use teloxide::dptree::{self, Handler, HandlerDescription, di::Injectable};

use crate::AuthenticationInfo;

pub trait AuthInfoInjector<'a, Output, Descr>
where
    Output: 'a,
    Descr: HandlerDescription,
{
    fn inject_auth_and_state<TState>(self) -> Handler<'a, Output, Descr>
    where
        TState: Clone + Send + Sync + 'static;
}

impl<'a, Output, Descr> AuthInfoInjector<'a, Output, Descr> for Handler<'a, Output, Descr>
where
    Output: 'a,
    Descr: HandlerDescription,
{
    fn inject_auth_and_state<TState>(self) -> Handler<'a, Output, Descr>
    where
        TState: Clone + Send + Sync + 'static,
    {
        self.map(|(a, _): (AuthenticationInfo, TState)| a)
            .map(|(_, b): (AuthenticationInfo, TState)| b)
    }
}
