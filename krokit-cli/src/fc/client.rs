use std::os::unix::net::UnixStream;
use std::path::Path;

use crate::fc::history::{CommandEntry, CommandHistory, HistoryStats};
use crate::fc::protocol::{KrokitProtocol, KrokitRequest, KrokitResponse, ResponseData};

/// Client for querying the command history via Unix socket
pub struct KrokitSessionClient {
    socket_path: String,
}

impl KrokitSessionClient {
    pub fn new(session_id: &str) -> Self {
        let socket_path = format!("/tmp/krokit_history_{}", session_id);
        Self { socket_path }
    }

    pub fn get_last_commands(&self, n: usize) -> Result<CommandHistory, Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| "Could not connect to KROKIT history session (is server running?)")?;
        
        let request = KrokitRequest::GetLastCmd { n };
        KrokitProtocol::write_request(&mut stream, &request)?;
        
        let response = KrokitProtocol::read_response(&mut stream)?;
        
        match response {
            KrokitResponse::Ok { data: ResponseData::Commands(entries) } => Ok(entries.into()),
            KrokitResponse::Ok { .. } => Err("Unexpected response type".into()),
            KrokitResponse::Error { message } => Err(message.into()),
        }
    }

    pub fn get_all_commands(&self) -> Result<CommandHistory, Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| "Could not connect to KROKIT history session (is server running?)")?;
        
        let request = KrokitRequest::GetAllCmd;
        KrokitProtocol::write_request(&mut stream, &request)?;
        
        let response = KrokitProtocol::read_response(&mut stream)?;
        
        match response {
            KrokitResponse::Ok { data: ResponseData::Commands(entries) } => Ok(entries.into()),
            KrokitResponse::Ok { .. } => Err("Unexpected response type".into()),
            KrokitResponse::Error { message } => Err(message.into()),
        }
    }

    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| "Could not connect to KROKIT history session (is server running?)")?;
        
        let request = KrokitRequest::Clear;
        KrokitProtocol::write_request(&mut stream, &request)?;
        
        let response = KrokitProtocol::read_response(&mut stream)?;
        
        match response {
            KrokitResponse::Ok { .. } => Ok(()),
            KrokitResponse::Error { message } => Err(message.into()),
        }
    }

    pub fn get_status(&self) -> Result<HistoryStats, Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| "Could not connect to KROKIT history session (is server running?)")?;
        
        let request = KrokitRequest::Status;
        KrokitProtocol::write_request(&mut stream, &request)?;
        
        let response = KrokitProtocol::read_response(&mut stream)?;
        
        match response {
            KrokitResponse::Ok { data: ResponseData::Stats(stats) } => Ok(stats),
            KrokitResponse::Ok { .. } => Err("Unexpected response type".into()),
            KrokitResponse::Error { message } => Err(message.into()),
        }
    }

    pub fn pre_command(&self, cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| "Could not connect to KROKIT history session (is server running?)")?;
        
        let request = KrokitRequest::PreCmd { cmd: cmd.to_string() };
        KrokitProtocol::write_request(&mut stream, &request)?;
        
        let response = KrokitProtocol::read_response(&mut stream)?;
        
        match response {
            KrokitResponse::Ok { .. } => Ok(()),
            KrokitResponse::Error { message } => Err(message.into()),
        }
    }

    pub fn post_command(&self, exit_code: i32,  cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| "Could not connect to KROKIT history session (is server running?)")?;
        
        let request = KrokitRequest::PostCmd { 
            cmd: cmd.to_string(), 
            exit_code
        };
        KrokitProtocol::write_request(&mut stream, &request)?;
        
        let response = KrokitProtocol::read_response(&mut stream)?;
        
        match response {
            KrokitResponse::Ok { .. } => Ok(()),
            KrokitResponse::Error { message } => Err(message.into()),
        }
    }

    pub fn session_exists(&self) -> bool {
        Path::new(&self.socket_path).exists()
    }
}

