use std::{collections::BTreeMap, net::IpAddr};

use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, SocketInfo, TcpState,
};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System, Uid, Users};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    cli::FilterArgs,
    error::{PortError, Result},
    model::{PortHolder, Protocol, SocketEntry},
};

#[derive(Debug, Clone)]
struct RawSocket {
    port: u16,
    proto: Protocol,
    pid: Option<u32>,
    bound: String,
    state: String,
    uid: Option<u32>,
}

pub fn inspect(filters: &FilterArgs, ports: &[u16]) -> Result<(Vec<SocketEntry>, Vec<PortHolder>)> {
    let raw = platform_sockets()?;
    let mut sockets = Vec::new();
    for socket in raw {
        if !ports.is_empty() && !ports.contains(&socket.port) {
            continue;
        }
        if !filters.proto.matches(socket.proto) {
            continue;
        }
        if let Some(host) = filters.host {
            if !bound_matches_host(&socket.bound, host) {
                continue;
            }
        }
        if !filters.include_loopback && is_loopback_bound(&socket.bound) {
            continue;
        }
        if filters.listening_only && socket.proto == Protocol::Tcp && socket.state != "LISTEN" {
            continue;
        }
        if let Some(pid) = filters.pid {
            if socket.pid != Some(pid) {
                continue;
            }
        }
        sockets.push(socket);
    }

    let needs_lookup = !sockets.is_empty();
    let system = needs_lookup.then(load_system);
    let users = needs_lookup.then(Users::new_with_refreshed_list);

    let mut filtered = Vec::with_capacity(sockets.len());
    for socket in sockets {
        let info = match (&system, &users, socket.pid) {
            (Some(sys), Some(usr), Some(pid)) => process_info(sys, usr, pid, socket.uid),
            _ => ProcessInfo::default(),
        };
        if let Some(owner) = &filters.owner {
            let socket_owner = info.owner.as_deref();
            if socket_owner != Some(owner.as_str()) {
                continue;
            }
        }
        filtered.push((socket, info));
    }

    filtered.sort_by(|left, right| {
        (left.0.port, left.0.proto, left.0.pid, &left.0.bound).cmp(&(
            right.0.port,
            right.0.proto,
            right.0.pid,
            &right.0.bound,
        ))
    });

    let mut entries = Vec::with_capacity(filtered.len());
    let mut groups: BTreeMap<(u16, Protocol, u32), PortHolder> = BTreeMap::new();
    for (socket, info) in filtered {
        entries.push(SocketEntry {
            port: socket.port,
            proto: socket.proto,
            pid: socket.pid,
            process: info.name.clone(),
            bound: socket.bound.clone(),
            state: socket.state.clone(),
        });
        let Some(pid) = socket.pid else {
            continue;
        };
        let key = (socket.port, socket.proto, pid);
        groups
            .entry(key)
            .and_modify(|holder| holder.bound.push(socket.bound.clone()))
            .or_insert_with(|| PortHolder {
                port: socket.port,
                proto: socket.proto,
                pid,
                parent_pid: info.parent_pid,
                name: info.name.clone().unwrap_or_else(|| "unknown".to_owned()),
                command: info.command,
                cwd: info.cwd,
                bound: vec![socket.bound.clone()],
                owner: info.owner,
                memory_bytes: info.memory_bytes,
                started: info.started,
            });
    }

    Ok((entries, groups.into_values().collect()))
}

#[derive(Debug, Default, Clone)]
struct ProcessInfo {
    name: Option<String>,
    command: Option<String>,
    cwd: Option<String>,
    owner: Option<String>,
    memory_bytes: Option<u64>,
    started: Option<String>,
    parent_pid: Option<u32>,
}

fn load_system() -> System {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );
    system
}

fn process_info(system: &System, users: &Users, pid: u32, socket_uid: Option<u32>) -> ProcessInfo {
    let process = system.process(Pid::from_u32(pid));
    let mut info = ProcessInfo::default();
    if let Some(process) = process {
        info.name = Some(process.name().to_string_lossy().into_owned());
        let cmd = process
            .cmd()
            .iter()
            .map(|part| part.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        info.command = (!cmd.is_empty()).then_some(cmd);
        info.cwd = process
            .cwd()
            .map(|path| path.to_string_lossy().into_owned());
        info.memory_bytes = Some(process.memory());
        info.parent_pid = process.parent().map(sysinfo::Pid::as_u32);
        info.started = format_unix_seconds(process.start_time());
        if let Some(uid) = process.user_id() {
            info.owner = lookup_user(users, uid);
        }
    }
    if info.owner.is_none() {
        if let Some(uid) = socket_uid {
            info.owner = lookup_user_by_raw(users, uid);
        }
    }
    info
}

fn lookup_user(users: &Users, uid: &Uid) -> Option<String> {
    users.get_user_by_id(uid).map(|user| user.name().to_owned())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn lookup_user_by_raw(users: &Users, uid: u32) -> Option<String> {
    users
        .list()
        .iter()
        .find(|user| **user.id() == uid)
        .map(|user| user.name().to_owned())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn lookup_user_by_raw(_users: &Users, _uid: u32) -> Option<String> {
    None
}

fn format_unix_seconds(seconds: u64) -> Option<String> {
    let secs = i64::try_from(seconds).ok()?;
    OffsetDateTime::from_unix_timestamp(secs)
        .ok()
        .and_then(|datetime| datetime.format(&Rfc3339).ok())
}

fn platform_sockets() -> Result<Vec<RawSocket>> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let infos = get_sockets_info(af_flags, proto_flags)
        .map_err(|err| PortError::Inspect(err.to_string()))?;
    Ok(infos.into_iter().map(convert_socket).collect())
}

fn convert_socket(info: SocketInfo) -> RawSocket {
    let uid = socket_uid(&info);
    let pid = info.associated_pids.into_iter().next();
    match info.protocol_socket_info {
        ProtocolSocketInfo::Tcp(tcp) => RawSocket {
            port: tcp.local_port,
            proto: Protocol::Tcp,
            pid,
            bound: format_endpoint(tcp.local_addr, tcp.local_port),
            state: tcp_state_label(tcp.state).to_owned(),
            uid,
        },
        ProtocolSocketInfo::Udp(udp) => RawSocket {
            port: udp.local_port,
            proto: Protocol::Udp,
            pid,
            bound: format_endpoint(udp.local_addr, udp.local_port),
            state: "UNCONN".to_owned(),
            uid,
        },
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn socket_uid(info: &SocketInfo) -> Option<u32> {
    Some(info.uid)
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn socket_uid(_info: &SocketInfo) -> Option<u32> {
    None
}

fn tcp_state_label(state: TcpState) -> &'static str {
    match state {
        TcpState::Listen => "LISTEN",
        TcpState::Established => "ESTABLISHED",
        TcpState::TimeWait => "TIME_WAIT",
        TcpState::SynSent => "SYN_SENT",
        TcpState::SynReceived => "SYN_RCVD",
        TcpState::FinWait1 => "FIN_WAIT_1",
        TcpState::FinWait2 => "FIN_WAIT_2",
        TcpState::CloseWait => "CLOSE_WAIT",
        TcpState::Closing => "CLOSING",
        TcpState::LastAck => "LAST_ACK",
        TcpState::DeleteTcb => "DELETE_TCB",
        TcpState::Closed => "CLOSED",
        TcpState::Unknown => "OTHER",
    }
}

fn format_endpoint(addr: IpAddr, port: u16) -> String {
    match addr {
        IpAddr::V4(value) => format!("{value}:{port}"),
        IpAddr::V6(value) => format!("[{value}]:{port}"),
    }
}

fn bound_matches_host(bound: &str, host: IpAddr) -> bool {
    let Some((addr, _port)) = bound.rsplit_once(':') else {
        return false;
    };
    let normalized = addr.trim_matches(['[', ']']);
    normalized == host.to_string() || (normalized == "*" && host.is_unspecified())
}

fn is_loopback_bound(bound: &str) -> bool {
    bound.starts_with("127.") || bound.starts_with("[::1]") || bound.starts_with("::1")
}
