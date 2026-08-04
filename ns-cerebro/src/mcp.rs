//! Hand-rolled MCP stdio client (newline-delimited JSON-RPC 2.0) — the same
//! shape Prefrontal's cortex client uses; no SDK dependency. The child is
//! killed on drop. cerebro-mcp double-encodes every tool result as JSON text
//! inside `result.content[0].text`; `call_tool` undoes that.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

use crate::{CerebroConfig, CerebroError};

pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpClient {
    pub fn spawn(cfg: &CerebroConfig) -> Result<Self, CerebroError> {
        let mut cmd = Command::new(&cfg.command);
        cmd.args(&cfg.args)
            .envs(&cfg.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .map_err(|e| CerebroError::Spawn(format!("{}: {e}", cfg.command)))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CerebroError::Spawn("no stdin pipe".into()))?;
        let stdout = child
            .stdout
            .take()
            .map(BufReader::new)
            .ok_or_else(|| CerebroError::Spawn("no stdout pipe".into()))?;

        let mut client = Self { child, stdin, stdout, next_id: 0 };
        // The first frame MUST be initialize or cerebro-mcp answers -32601.
        client.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "neuralsymphony", "version": env!("CARGO_PKG_VERSION") }
            }),
        )?;
        Ok(client)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, CerebroError> {
        self.next_id += 1;
        let id = self.next_id;
        let frame = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{frame}")
            .map_err(|e| CerebroError::Protocol(format!("write: {e}")))?;
        self.stdin
            .flush()
            .map_err(|e| CerebroError::Protocol(format!("flush: {e}")))?;

        let mut line = String::new();
        loop {
            line.clear();
            let n = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| CerebroError::Protocol(format!("read: {e}")))?;
            if n == 0 {
                return Err(CerebroError::Protocol("cerebro-mcp closed the pipe".into()));
            }
            if line.trim().is_empty() {
                continue;
            }
            let msg: Value = serde_json::from_str(&line)
                .map_err(|e| CerebroError::Protocol(format!("bad frame: {e}")))?;
            if msg.get("id").and_then(Value::as_i64) != Some(id) {
                continue; // stray notification or stale frame — not ours
            }
            if let Some(err) = msg.get("error") {
                let text = err.get("message").and_then(Value::as_str).unwrap_or("unknown");
                return Err(CerebroError::Tool(text.to_string()));
            }
            return msg
                .get("result")
                .cloned()
                .ok_or_else(|| CerebroError::Protocol("frame with neither result nor error".into()));
        }
    }

    /// Call a tool and decode the double-encoded payload.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, CerebroError> {
        let result =
            self.request("tools/call", json!({ "name": name, "arguments": arguments }))?;
        let text = result
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CerebroError::Protocol(format!("{name}: result missing content[0].text"))
            })?;
        serde_json::from_str(text)
            .map_err(|e| CerebroError::Protocol(format!("{name}: payload not JSON: {e}")))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
