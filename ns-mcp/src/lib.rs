//! Hand-rolled MCP stdio client (newline-delimited JSON-RPC 2.0) — the
//! prefrontal pattern, extracted once so every bridge (cerebro, sonus)
//! speaks it the same way. Servers that double-encode tool results as JSON
//! text inside `result.content[0].text` are undone by `call_tool`; servers
//! that return plain text keep it via `call_tool_text`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

#[derive(Debug)]
pub enum McpError {
    Spawn(String),
    Protocol(String),
    /// The tool ran and answered isError / a JSON-RPC error.
    Tool(String),
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::Spawn(e) => write!(f, "spawn: {e}"),
            McpError::Protocol(e) => write!(f, "protocol: {e}"),
            McpError::Tool(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for McpError {}

pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpClient {
    pub fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self, McpError> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::Spawn(format!("{command}: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Spawn("no stdin pipe".into()))?;
        let stdout = child
            .stdout
            .take()
            .map(BufReader::new)
            .ok_or_else(|| McpError::Spawn("no stdout pipe".into()))?;

        let mut client = Self { child, stdin, stdout, next_id: 0 };
        // Many servers require initialize as the very first frame.
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

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError> {
        self.next_id += 1;
        let id = self.next_id;
        let frame = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{frame}").map_err(|e| McpError::Protocol(format!("write: {e}")))?;
        self.stdin.flush().map_err(|e| McpError::Protocol(format!("flush: {e}")))?;

        let mut line = String::new();
        loop {
            line.clear();
            let n = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| McpError::Protocol(format!("read: {e}")))?;
            if n == 0 {
                return Err(McpError::Protocol("server closed the pipe".into()));
            }
            if line.trim().is_empty() {
                continue;
            }
            let msg: Value = serde_json::from_str(&line)
                .map_err(|e| McpError::Protocol(format!("bad frame: {e}")))?;
            if msg.get("id").and_then(Value::as_i64) != Some(id) {
                continue; // stray notification or stale frame — not ours
            }
            if let Some(err) = msg.get("error") {
                let text = err.get("message").and_then(Value::as_str).unwrap_or("unknown");
                return Err(McpError::Tool(text.to_string()));
            }
            return msg
                .get("result")
                .cloned()
                .ok_or_else(|| McpError::Protocol("frame with neither result nor error".into()));
        }
    }

    /// Call a tool; error on isError results, return the raw content text.
    pub fn call_tool_text(&mut self, name: &str, arguments: Value) -> Result<String, McpError> {
        let result = self.request("tools/call", json!({ "name": name, "arguments": arguments }))?;
        let text = result
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::Protocol(format!("{name}: result missing content[0].text")))?
            .to_string();
        if result.get("isError").and_then(Value::as_bool).unwrap_or(false) {
            return Err(McpError::Tool(format!("{name}: {text}")));
        }
        Ok(text)
    }

    /// Call a tool and parse the double-encoded JSON payload.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, McpError> {
        let text = self.call_tool_text(name, arguments)?;
        serde_json::from_str(&text)
            .map_err(|e| McpError::Protocol(format!("{name}: payload not JSON: {e}")))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
