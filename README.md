# Nadoeda

`nadoeda` is a Rust-powered reminder system that not just reminds: it **annoys** you until you take action. In addition, it asks you if you *really* did what you were supposed to do. It will support one-off and recurring daily reminders.

## ✨ Features

- 🔁 **One-time & Daily Reminders**: Schedule both ad-hoc and recurring daily reminders.
- 📣 **Multichannel Messaging**: Supports Telegram (more channels coming soon).
- 🔔 **Nag Mode**: Repeatedly reminds users until they acknowledge the message.
- ✅ **Completion Confirmation**: Sends a follow-up message after a delay to verify task completion.
  
## 📦 Libraries Used
- [`tokio`](https://crates.io/crates/tokio) — for async runtime and task scheduling
- [`teloxide`](https://crates.io/crates/teloxide) — for Telegram bot integration
- [`sqlx`](https://github.com/launchbadge/sqlx) - for storage management
