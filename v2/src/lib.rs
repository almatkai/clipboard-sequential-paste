pub mod store;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use store::{Pending, Queue, Store};
use tezbar_extension_sdk::{
    no_view, unchanged, view, BoxFuture, CoreResult, Extension, ExtensionContext,
};
use tokio::sync::Mutex as AsyncMutex;

const DEBOUNCE: Duration = Duration::from_millis(650);
const HOST_TIMEOUT: Duration = Duration::from_secs(10);
#[derive(Clone, Copy)]
enum Choice {
    Retry,
    Advance,
    Continue,
    New,
}
struct Ticket {
    session: String,
    revision: u64,
    choices: Vec<(String, Choice)>,
    done: Option<tokio::sync::oneshot::Sender<()>>,
}
type Tickets = Arc<Mutex<HashMap<String, Ticket>>>;
struct Registration {
    id: String,
    tickets: Tickets,
}
impl Drop for Registration {
    fn drop(&mut self) {
        if let Ok(mut t) = self.tickets.lock() {
            t.remove(&self.id);
        }
    }
}
#[derive(Default)]
struct Runtime {
    data: Option<PathBuf>,
    admitted: Option<Instant>,
    global_index: usize,
}
pub struct Adapter {
    gate: AsyncMutex<Runtime>,
    tickets: Tickets,
    owner: String,
}
impl Default for Adapter {
    fn default() -> Self {
        Self {
            gate: AsyncMutex::default(),
            tickets: Arc::default(),
            owner: uuid::Uuid::new_v4().to_string(),
        }
    }
}
fn error_view(message: &str) -> Value {
    let mut v = view(
        "Clipboard Sequential Paste",
        json!({"type":"Detail","props":{"markdown":format!("# Paste stopped\n\n{message}\n\nNothing will retry automatically. Close this view and run Paste Next Item after resolving the problem.")}}),
        vec![],
    );
    v["refreshMs"] = Value::Null;
    v
}
fn ack(value: &Value) -> bool {
    value.as_object().is_some_and(|o| {
        o.get("ok") == Some(&Value::Bool(true))
            && o.keys().all(|k| k == "ok" || k == "message")
            && o.get("message").is_none_or(Value::is_string)
    })
}
fn wall_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "System clock precedes Unix epoch".into())
        .and_then(|d| u64::try_from(d.as_millis()).map_err(|_| "System clock out of range".into()))
}
impl Adapter {
    fn recover(
        &self,
        ctx: &ExtensionContext,
        payload: &Value,
        q: &Queue,
        unknown: bool,
    ) -> CoreResult {
        let session = payload["sessionId"]
            .as_str()
            .ok_or("Missing SDK session")?
            .to_owned();
        let id = uuid::Uuid::new_v4().to_string();
        let labels = if unknown {
            vec![
                ("Retry exact saved item (may duplicate)", Choice::Retry),
                ("Mark item handled and advance (may skip)", Choice::Advance),
            ]
        } else {
            vec![
                ("Continue saved list", Choice::Continue),
                ("Use current clipboard as a new list", Choice::New),
            ]
        };
        let choices: Vec<_> = labels
            .iter()
            .map(|(_, c)| (uuid::Uuid::new_v4().to_string(), *c))
            .collect();
        let (done, finished) = tokio::sync::oneshot::channel();
        {
            let mut map = self
                .tickets
                .lock()
                .map_err(|_| "Recovery registry unavailable")?;
            if map.len() >= 128 {
                return Err("Recovery session limit reached; close old views".into());
            }
            map.insert(
                id.clone(),
                Ticket {
                    session,
                    revision: q.revision,
                    choices: choices.clone(),
                    done: Some(done),
                },
            );
        }
        let registration = Registration {
            id,
            tickets: self.tickets.clone(),
        };
        // Construct the guard before the future: cancellation before its first poll
        // still drops the authoritative registration. SDK owns and cancels this
        // inert lifecycle guard on operation cancel/session dispose/EOF.
        ctx.spawn(async move {
            let _registration = registration;
            let _ = finished.await;
        })?;
        let text = if unknown {
            "# Paste result is uncertain\n\nThe saved item may already have been pasted. No item was automatically repeated or skipped. Choose Retry only if a duplicate is acceptable, or Mark handled only if a skipped item is acceptable. Retry uses the exact saved bytes without reading the clipboard.\n\nEnter chooses Retry. Cmd+K opens both choices."
        } else {
            "# Clipboard ownership is ambiguous\n\nThe clipboard equals the last produced item from an earlier process. It could also be a newly copied identical value. Continue uses the retained original list; Use current clipboard starts from the current clipboard.\n\nEnter continues the saved list. Cmd+K opens both choices."
        };
        let actions = choices
            .iter()
            .zip(labels)
            .map(|((id, _), (title, _))| json!({"id":id,"title":title,"kind":"action"}))
            .collect();
        let mut v = view(
            "Clipboard Sequential Paste — recovery",
            json!({"type":"Detail","props":{"markdown":text}}),
            actions,
        );
        v["refreshMs"] = Value::Null;
        Ok(v)
    }
    fn take_ticket(&self, payload: &Value) -> Result<Ticket, String> {
        let action = payload["actionId"]
            .as_str()
            .ok_or("Missing recovery action")?;
        let session = payload["sessionId"].as_str().ok_or("Missing SDK session")?;
        let mut map = self
            .tickets
            .lock()
            .map_err(|_| "Recovery registry unavailable")?;
        let key = map
            .iter()
            .find(|(_, t)| t.session == session && t.choices.iter().any(|(id, _)| id == action))
            .map(|(k, _)| k.clone())
            .ok_or("Recovery action expired or already used; close and reopen")?;
        let mut ticket = map.remove(&key).ok_or("Recovery action already used")?;
        if let Some(done) = ticket.done.take() {
            let _ = done.send(());
        }
        Ok(ticket)
    }
    async fn host(&self, ctx: &ExtensionContext, method: &str, params: Value) -> CoreResult {
        tokio::time::timeout(HOST_TIMEOUT, ctx.host(method, params))
            .await
            .map_err(|_| "Host request timed out".to_string())?
    }
    async fn close_without_paste(&self, ctx: &ExtensionContext, message: &str) -> CoreResult {
        // Source closeMainWindow on empty/debounced launches. Unlike pasteText's
        // structured ACK, the exact pinned visibility handler returns JSON null.
        let result = self
            .host(ctx, "host.window.visibility", json!({"visible": false}))
            .await
            .map_err(|_| "Window hide failed; no paste was attempted".to_string())?;
        if !result.is_null() {
            return Err("Unexpected window visibility response; no paste attempted".into());
        }
        Ok(no_view(message))
    }
    async fn paste(
        &self,
        ctx: &ExtensionContext,
        payload: &Value,
        store: &Store,
        q: &mut Queue,
    ) -> CoreResult {
        let text = q.pending.as_ref().ok_or("No pending effect")?.text.clone();
        // The pending checkpoint is already durable. Every failure, malformed ACK
        // and dropped future leaves it pending. Host and store are not a transaction.
        let result = self
            .host(ctx, "host.clipboard.pasteText", json!({"text":text}))
            .await;
        if !result.as_ref().is_ok_and(ack) {
            return self.recover(ctx, payload, q, true);
        }
        let mut done = q.clone();
        done.advance(&self.owner)?;
        if store.save(&mut done).is_err() {
            return Ok(error_view("Host acknowledged paste, but its completion could not be durably recorded. The saved checkpoint may remain pending. Reopen to inspect it; do not assume the item failed."));
        }
        *q = done;
        Ok(no_view(if q.active {
            "Pasted next item"
        } else {
            "Pasted every item from list"
        }))
    }
    async fn run(
        &self,
        ctx: &ExtensionContext,
        payload: &Value,
        arrival: Instant,
        wall: u64,
        ticket: Option<Ticket>,
    ) -> CoreResult {
        let mut rt = self.gate.lock().await;
        if rt.data.as_ref().is_some_and(|p| p != &ctx.paths.data_dir) {
            return Err("A process cannot change profile data directory".into());
        }
        rt.data = Some(ctx.paths.data_dir.clone());
        let store = Store::new(&ctx.paths.data_dir)?;
        let mut q = store.load()?;
        let choice = if let Some(ticket) = ticket {
            if ticket.revision != q.revision {
                return Err("Queue changed; recovery action expired. Close and reopen.".into());
            }
            let action = payload["actionId"].as_str().ok_or("Missing action")?;
            Some(
                ticket
                    .choices
                    .iter()
                    .find(|(id, _)| id == action)
                    .ok_or("Unknown action")?
                    .1,
            )
        } else {
            None
        };
        if let Some(choice) = choice {
            rt.admitted = Some(arrival);
            q.last_wall_ms = wall;
            match choice {
                Choice::Retry => {
                    if q.pending.is_none() {
                        return Err("No pending item".into());
                    }
                    store.save(&mut q)?; // revision fences other views before effect
                    return self.paste(ctx, payload, &store, &mut q).await;
                }
                Choice::Advance => {
                    q.advance(&self.owner)?;
                    store.save(&mut q)?;
                    return self
                        .close_without_paste(
                            ctx,
                            "Item marked handled; run again for the next item",
                        )
                        .await;
                }
                Choice::Continue | Choice::New => {}
            }
        } else {
            if q.pending.is_some() {
                return self.recover(ctx, payload, &q, true);
            }
            // Capture arrival before waiting for the process-wide gate. A queued
            // hotkey inside the 650ms window must not become a new paste later.
            let suppressed = match rt.admitted {
                Some(last) => arrival
                    .checked_duration_since(last)
                    .is_none_or(|d| d < DEBOUNCE),
                None => wall >= q.last_wall_ms && wall - q.last_wall_ms < 650,
            };
            if suppressed {
                return self.close_without_paste(ctx, "Debounced (650 ms)").await;
            }
            rt.admitted = Some(arrival);
            q.last_wall_ms = wall;
            store.save(&mut q)?;
        }
        let clipboard = self
            .host(ctx, "host.clipboard.read", json!({}))
            .await
            .map_err(|_| {
                "Clipboard read failed. Queue retained; check host clipboard access and run again."
                    .to_owned()
            })?;
        let text = clipboard
            .as_str()
            .ok_or("Host clipboard.read must return a string; queue retained")?;
        let same_produced = q.produced.as_deref() == Some(text);
        if choice.is_none() && same_produced && q.owner.as_deref() != Some(&self.owner) {
            return self.recover(ctx, payload, &q, false);
        }
        let effective =
            if matches!(choice, Some(Choice::Continue)) || (choice.is_none() && same_produced) {
                q.original.as_deref().ok_or("Missing retained original")?
            } else {
                text
            };
        if effective.is_empty() {
            return self.close_without_paste(ctx, "Clipboard is empty.").await;
        }
        let lines = store::split(effective)?;
        let old_list = q.active
            && q.original.as_deref() == Some(effective)
            && !matches!(choice, Some(Choice::New));
        let index = if old_list { q.next } else { rt.global_index };
        let item = lines.get(index).ok_or("Source retained its global index, which exceeds the new list length. Queue retained; close this process and run again to reset its in-memory index.")?.clone();
        let mut prepared = q.clone();
        prepared.original = Some(effective.into());
        prepared.active = true;
        prepared.next = index;
        prepared.pending = Some(Pending { text: item, index });
        store.save(&mut prepared)?;
        rt.global_index = index;
        self.paste(ctx, payload, &store, &mut prepared).await
    }
}
impl Extension for Adapter {
    fn id(&self) -> &'static str {
        "com.tezbar.native.clipboard-sequential-paste"
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn commands(&self) -> &'static [&'static str] {
        &["paste-next-item"]
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        operation: &'a str,
        payload: &'a Value,
        _state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        let arrival = Instant::now();
        // Consume authoritative actions synchronously, before the first await.
        let ticket = if operation == "extension:invoke-action" {
            self.take_ticket(payload).map(Some)
        } else {
            Ok(None)
        };
        Box::pin(async move {
            if command != "paste-next-item" {
                return Err("Unknown command".into());
            }
            if operation == "extension:dispose-session" {
                return Ok(no_view("Disposed"));
            }
            if operation == "extension:refresh-session" {
                return Ok(unchanged());
            }
            if operation != "extension:run-command" && operation != "extension:invoke-action" {
                return Err("Unsupported operation".into());
            }
            let result = match (ticket, wall_ms()) {
                (Ok(ticket), Ok(wall)) => self.run(ctx, payload, arrival, wall, ticket).await,
                (Err(e), _) | (_, Err(e)) => Err(e),
            };
            Ok(result.unwrap_or_else(|e| error_view(&e)))
        })
    }
}

#[cfg(test)]
mod tests {
    mod domain;
    use super::*;
    #[test]
    fn closed_ack() {
        for v in [
            json!(null),
            json!(true),
            json!({"ok":false}),
            json!({"ok":true,"error":"bad"}),
            json!({"ok":true,"message":null}),
            json!({"ok":true,"x":1}),
        ] {
            assert!(!ack(&v));
        }
        assert!(ack(&json!({"ok":true})));
        assert!(ack(&json!({"ok":true,"message":"Pasted text"})));
    }
    #[test]
    fn exact_newlines_bounds() {
        assert_eq!(
            store::split("a\r\n\n\r\nz\r").unwrap(),
            ["a", "", "", "z\r"]
        );
        assert_eq!(store::split("x\n").unwrap(), ["x", ""]);
        assert!(store::split(&"x".repeat(store::MAX_LINE + 1)).is_err());
        assert!(store::split(&"\n".repeat(store::MAX_LINES)).is_err());
        assert!(store::split(&"x".repeat(store::MAX_INPUT + 1)).is_err());
    }
}
