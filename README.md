# annoying_reminders

`annoying_reminders` is a Rust-powered reminder system that not just reminds — it **nags** you until you take action. In addition, it asks you if you *really* did what you were supposed to do. It supports one-off and recurring daily reminders.

## ✨ Features

- 🔁 **One-time & Daily Reminders**: Schedule both ad-hoc and recurring daily reminders.
- 📣 **Multichannel Messaging**: Supports Telegram and Discord (more channels coming soon).
- 🔔 **Nag Mode**: Repeatedly reminds users until they acknowledge the message.
- ✅ **Completion Confirmation**: Sends a follow-up message after a configurable delay to verify task completion.
  
## 📦 Libraries Used

- [`tokio`](https://crates.io/crates/tokio) — for async runtime and task scheduling
- [`teloxide`](https://crates.io/crates/teloxide) — for Telegram bot integration
