use std::default;

use dptree::case;
use teloxide::dispatching::{UpdateHandler, dialogue};
use teloxide::{handler, prelude::*};

use super::{GlobalCommand, GlobalDialogue, GlobalState, HandlerResult, HandlerStorageType};

#[derive(Clone, Default)]
pub(super) enum EditRemindersState {
    #[default]
    ListReminders,
}

async fn list_reminders(
    storage: HandlerStorageType,
    bot: Bot,
    dialogue: GlobalDialogue,
    msg: Message,
) -> HandlerResult {
    let reminders = storage.get_all();
    
    todo!()
}
pub(super) fn schema() -> UpdateHandler<anyhow::Error> {
    dptree::entry().branch(
        Update::filter_message().branch(
            teloxide::filter_command::<GlobalCommand, _>().branch(
                case![GlobalState::Idle]
                    .branch(case![GlobalCommand::ListReminders].endpoint(list_reminders)),
            ),
        ),
    )
}
