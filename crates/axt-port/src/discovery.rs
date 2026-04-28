use std::{collections::BTreeMap, net::IpAddr, process::Command};

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
    name: Option<String>,
    bound: String,
    state: String,
    owner: Option<String>,
    parent_pid: Option<u32>,
}

pub fn inspect(filters: &FilterArgs, ports: &[u16]) -> Result<(Vec<SocketEntry>, Vec<PortHolder>)> {
    let raw = platform_sockets(filters)?;
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
        if let Some(owner) = &filters.owner {
            if socket.owner.as_deref() != Some(owner.as_str()) {
                continue;
            }
        }
        sockets.push(socket);
    }
    sockets.sort_by(|left, right| {
        (left.port, left.proto, left.pid, &left.bound).cmp(&(
            right.port,
            right.proto,
            right.pid,
            &right.bound,
        ))
    });

    let mut entries = Vec::with_capacity(sockets.len());
    let mut groups: BTreeMap<(u16, Protocol, u32), PortHolder> = BTreeMap::new();
    for socket in sockets {
        entries.push(SocketEntry {
            port: socket.port,
            proto: socket.proto,
            pid: socket.pid,
            process: socket.name.clone(),
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
            .or_insert_with(|| process_holder(&socket, pid));
    }

    Ok((entries, groups.into_values().collect()))
}

fn process_holder(socket: &RawSocket, pid: u32) -> PortHolder {
    let info = process_info(pid);
    PortHolder {
        port: socket.port,
        proto: socket.proto,
        pid,
        parent_pid: socket.parent_pid.or(info.parent_pid),
        name: socket
            .name
            .clone()
            .or(info.name)
            .unwrap_or_else(|| "unknown".to_owned()),
        command: info.command,
        cwd: info.cwd,
        bound: vec![socket.bound.clone()],
        owner: socket.owner.clone().or(info.owner),
        memory_bytes: info.memory_bytes,
        started: info.started,
    }
}

#[derive(Debug, Default)]
struct ProcessInfo {
    name: Option<String>,
    command: Option<String>,
    cwd: Option<String>,
    owner: Option<String>,
    memory_bytes: Option<u64>,
    started: Option<String>,
    parent_pid: Option<u32>,
}

#[cfg(target_os = "linux")]
fn platform_sockets(_filters: &FilterArgs) -> Result<Vec<RawSocket>> {
    linux::sockets()
}

#[cfg(target_os = "macos")]
fn platform_sockets(_filters: &FilterArgs) -> Result<Vec<RawSocket>> {
    lsof_sockets()
}

#[cfg(windows)]
fn platform_sockets(_filters: &FilterArgs) -> Result<Vec<RawSocket>> {
    windows_netstat_sockets()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_sockets(_filters: &FilterArgs) -> Result<Vec<RawSocket>> {
    Err(PortError::Inspect(
        "socket inspection is unsupported on this platform".to_owned(),
    ))
}

#[cfg(target_os = "macos")]
fn process_info(pid: u32) -> ProcessInfo {
    ProcessInfo {
        name: ps_field(pid, "comm="),
        command: ps_field(pid, "command="),
        cwd: None,
        owner: None,
        memory_bytes: ps_field(pid, "rss=")
            .and_then(|rss| rss.parse::<u64>().ok())
            .map(|rss| rss.saturating_mul(1024)),
        started: None,
        parent_pid: ps_field(pid, "ppid=").and_then(|value| value.parse::<u32>().ok()),
    }
}

#[cfg(target_os = "macos")]
fn ps_field(pid: u32, field: &str) -> Option<String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", field])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(windows)]
fn process_info(pid: u32) -> ProcessInfo {
    let script = format!(
        r#"$p=Get-CimInstance Win32_Process -Filter "ProcessId={pid}"; if ($p) {{ $owner=$null; try {{ $o=Invoke-CimMethod -InputObject $p -MethodName GetOwner; if ($o.ReturnValue -eq 0) {{ $owner="$($o.Domain)\$($o.User)" }} }} catch {{ }}; [pscustomobject]@{{ Name=$p.Name; CommandLine=$p.CommandLine; ParentProcessId=$p.ParentProcessId; WorkingSetSize=$p.WorkingSetSize; Owner=$owner }} | ConvertTo-Json -Compress }}"#
    );
    let Ok(output) = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
    else {
        return ProcessInfo::default();
    };
    if !output.status.success() || output.stdout.is_empty() {
        return ProcessInfo::default();
    }
    parse_windows_process_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(windows)]
fn parse_windows_process_json(text: &str) -> ProcessInfo {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text.trim()) else {
        return ProcessInfo::default();
    };
    ProcessInfo {
        name: value
            .get("Name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        command: value
            .get("CommandLine")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        cwd: None,
        owner: value
            .get("Owner")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        memory_bytes: value.get("WorkingSetSize").and_then(json_u64),
        started: None,
        parent_pid: value
            .get("ParentProcessId")
            .and_then(json_u64)
            .and_then(|pid| u32::try_from(pid).ok()),
    }
}

#[cfg(windows)]
fn json_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
}

#[cfg(target_os = "linux")]
fn process_info(pid: u32) -> ProcessInfo {
    linux::process_info(pid)
}

#[cfg(target_os = "macos")]
fn lsof_sockets() -> Result<Vec<RawSocket>> {
    let mut sockets = Vec::new();
    sockets.extend(run_lsof(Protocol::Tcp)?);
    sockets.extend(run_lsof(Protocol::Udp)?);
    Ok(sockets)
}

#[cfg(target_os = "macos")]
fn run_lsof(proto: Protocol) -> Result<Vec<RawSocket>> {
    let mut command = Command::new("lsof");
    command.args(["-nP", "-F", "pcnRuL"]);
    match proto {
        Protocol::Tcp => {
            command.args(["-iTCP", "-sTCP:LISTEN"]);
        }
        Protocol::Udp => {
            command.arg("-iUDP");
        }
    }
    let output = command.output().map_err(|source| PortError::Command {
        command: "lsof",
        source,
    })?;
    if !output.status.success() && output.stdout.is_empty() {
        return Ok(Vec::new());
    }
    Ok(parse_lsof(&String::from_utf8_lossy(&output.stdout), proto))
}

#[cfg(target_os = "macos")]
fn parse_lsof(text: &str, proto: Protocol) -> Vec<RawSocket> {
    let mut sockets = Vec::new();
    let mut pid = None;
    let mut parent_pid = None;
    let mut name = None;
    let mut owner = None;
    for line in text.lines() {
        let Some((prefix, value)) = line.split_at_checked(1) else {
            continue;
        };
        match prefix {
            "p" => pid = value.parse::<u32>().ok(),
            "R" => parent_pid = value.parse::<u32>().ok(),
            "c" => name = Some(value.to_owned()),
            "L" => owner = Some(value.to_owned()),
            "n" => {
                if let Some(port) = parse_bound_port(value) {
                    sockets.push(RawSocket {
                        port,
                        proto,
                        pid,
                        name: name.clone(),
                        bound: value.to_owned(),
                        state: if proto == Protocol::Tcp {
                            "LISTEN".to_owned()
                        } else {
                            "UNCONN".to_owned()
                        },
                        owner: owner.clone(),
                        parent_pid,
                    });
                }
            }
            _ => {}
        }
    }
    sockets
}

#[cfg(windows)]
fn windows_netstat_sockets() -> Result<Vec<RawSocket>> {
    let output = Command::new("netstat")
        .args(["-ano"])
        .output()
        .map_err(|source| PortError::Command {
            command: "netstat",
            source,
        })?;
    if !output.status.success() {
        return Err(PortError::Inspect(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let mut sockets = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 4 {
            continue;
        }
        let proto = match fields[0].to_ascii_lowercase().as_str() {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            _ => continue,
        };
        let (bound, state, pid_field) = if proto == Protocol::Tcp && fields.len() >= 5 {
            (fields[1], fields[3], fields[4])
        } else {
            (fields[1], "UNCONN", fields[3])
        };
        let Some(port) = parse_bound_port(bound) else {
            continue;
        };
        sockets.push(RawSocket {
            port,
            proto,
            pid: pid_field.parse::<u32>().ok(),
            name: None,
            bound: bound.to_owned(),
            state: state.to_owned(),
            owner: None,
            parent_pid: None,
        });
    }
    Ok(sockets)
}

fn parse_bound_port(value: &str) -> Option<u16> {
    value
        .rsplit_once(':')
        .and_then(|(_, port)| port.trim_matches(']').parse::<u16>().ok())
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

#[cfg(target_os = "linux")]
mod linux {
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        os::unix::fs::MetadataExt,
        path::Path,
    };

    use super::{ProcessInfo, Protocol, RawSocket};
    use crate::{error::PortError, error::Result};

    pub fn sockets() -> Result<Vec<RawSocket>> {
        let inode_pids = inode_pids();
        let users = passwd_users();
        let mut sockets = Vec::new();
        sockets.extend(parse_proc_net(
            "/proc/net/tcp",
            Protocol::Tcp,
            false,
            &inode_pids,
            &users,
        )?);
        sockets.extend(parse_proc_net(
            "/proc/net/tcp6",
            Protocol::Tcp,
            true,
            &inode_pids,
            &users,
        )?);
        sockets.extend(parse_proc_net(
            "/proc/net/udp",
            Protocol::Udp,
            false,
            &inode_pids,
            &users,
        )?);
        sockets.extend(parse_proc_net(
            "/proc/net/udp6",
            Protocol::Udp,
            true,
            &inode_pids,
            &users,
        )?);
        Ok(sockets)
    }

    pub fn process_info(pid: u32) -> ProcessInfo {
        let mut info = ProcessInfo::default();
        let base = format!("/proc/{pid}");
        info.name = fs::read_to_string(format!("{base}/comm"))
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        info.command = fs::read(format!("{base}/cmdline"))
            .ok()
            .map(|bytes| {
                bytes
                    .split(|byte| *byte == 0)
                    .filter(|part| !part.is_empty())
                    .map(|part| String::from_utf8_lossy(part).into_owned())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|value| !value.is_empty());
        info.cwd = fs::read_link(format!("{base}/cwd"))
            .ok()
            .map(|path| path.to_string_lossy().into_owned());
        info.memory_bytes = read_status_value(pid, "VmRSS:").map(|kb| kb.saturating_mul(1024));
        info.parent_pid =
            read_status_value(pid, "PPid:").and_then(|value| u32::try_from(value).ok());
        if let Ok(metadata) = fs::metadata(&base) {
            info.owner = passwd_users().get(&metadata.uid()).cloned();
        }
        info
    }

    fn parse_proc_net(
        path: &str,
        proto: Protocol,
        ipv6: bool,
        inode_pids: &BTreeMap<u64, BTreeSet<u32>>,
        users: &BTreeMap<u32, String>,
    ) -> Result<Vec<RawSocket>> {
        let text = fs::read_to_string(path).map_err(|source| PortError::Command {
            command: "procfs",
            source,
        })?;
        let mut sockets = Vec::new();
        for line in text.lines().skip(1) {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() <= 9 {
                continue;
            }
            let Some((host, port)) = parse_local_address(fields[1], ipv6) else {
                continue;
            };
            let state = tcp_state(fields[3], proto).to_owned();
            let uid = fields[7].parse::<u32>().ok();
            let inode = fields[9].parse::<u64>().ok();
            let pids = inode
                .and_then(|inode| inode_pids.get(&inode))
                .cloned()
                .unwrap_or_default();
            if pids.is_empty() {
                sockets.push(RawSocket {
                    port,
                    proto,
                    pid: None,
                    name: None,
                    bound: format!("{host}:{port}"),
                    state,
                    owner: uid.and_then(|uid| users.get(&uid).cloned()),
                    parent_pid: None,
                });
                continue;
            }
            for pid in pids {
                let info = process_info(pid);
                sockets.push(RawSocket {
                    port,
                    proto,
                    pid: Some(pid),
                    name: info.name,
                    bound: format!("{host}:{port}"),
                    state: state.clone(),
                    owner: uid.and_then(|uid| users.get(&uid).cloned()),
                    parent_pid: info.parent_pid,
                });
            }
        }
        Ok(sockets)
    }

    fn inode_pids() -> BTreeMap<u64, BTreeSet<u32>> {
        let mut map = BTreeMap::<u64, BTreeSet<u32>>::new();
        let Ok(entries) = fs::read_dir("/proc") else {
            return map;
        };
        for entry in entries.flatten() {
            let Some(pid) = entry.file_name().to_string_lossy().parse::<u32>().ok() else {
                continue;
            };
            let fd_dir = entry.path().join("fd");
            let Ok(fds) = fs::read_dir(fd_dir) else {
                continue;
            };
            for fd in fds.flatten() {
                let Ok(target) = fs::read_link(fd.path()) else {
                    continue;
                };
                if let Some(inode) = socket_inode(&target) {
                    map.entry(inode).or_default().insert(pid);
                }
            }
        }
        map
    }

    fn socket_inode(path: &Path) -> Option<u64> {
        let text = path.to_string_lossy();
        text.strip_prefix("socket:[")
            .and_then(|value| value.strip_suffix(']'))
            .and_then(|value| value.parse::<u64>().ok())
    }

    fn parse_local_address(value: &str, ipv6: bool) -> Option<(String, u16)> {
        let (host_hex, port_hex) = value.split_once(':')?;
        let port = u16::from_str_radix(port_hex, 16).ok()?;
        let host = if ipv6 {
            parse_ipv6(host_hex)
        } else {
            parse_ipv4(host_hex)
        }?;
        Some((host, port))
    }

    fn parse_ipv4(hex: &str) -> Option<String> {
        if hex.len() != 8 {
            return None;
        }
        let raw = u32::from_str_radix(hex, 16).ok()?;
        let bytes = raw.to_le_bytes();
        Some(format!(
            "{}.{}.{}.{}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        ))
    }

    fn parse_ipv6(hex: &str) -> Option<String> {
        if hex.len() != 32 {
            return None;
        }
        let mut bytes = [0_u8; 16];
        for index in 0..4 {
            let chunk = u32::from_str_radix(&hex[index * 8..index * 8 + 8], 16).ok()?;
            bytes[index * 4..index * 4 + 4].copy_from_slice(&chunk.to_le_bytes());
        }
        Some(std::net::Ipv6Addr::from(bytes).to_string())
    }

    fn tcp_state(value: &str, proto: Protocol) -> &'static str {
        if proto == Protocol::Udp {
            return "UNCONN";
        }
        match value {
            "0A" => "LISTEN",
            "01" => "ESTABLISHED",
            "06" => "TIME_WAIT",
            _ => "OTHER",
        }
    }

    fn passwd_users() -> BTreeMap<u32, String> {
        let mut users = BTreeMap::new();
        let Ok(text) = fs::read_to_string("/etc/passwd") else {
            return users;
        };
        for line in text.lines() {
            let fields = line.split(':').collect::<Vec<_>>();
            if fields.len() > 2 {
                if let Ok(uid) = fields[2].parse::<u32>() {
                    users.insert(uid, fields[0].to_owned());
                }
            }
        }
        users
    }

    fn read_status_value(pid: u32, key: &str) -> Option<u64> {
        let text = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        text.lines().find_map(|line| {
            line.strip_prefix(key).and_then(|value| {
                value
                    .split_whitespace()
                    .next()
                    .and_then(|number| number.parse::<u64>().ok())
            })
        })
    }
}
