use std::path::Path;

use chrono::{DateTime, Utc};
use exec_rs::Exec;

use crate::{ssh_credentials::SshCredentials, sync_error::SyncError};

/// run rsync to synchronize the local files with the files on the server
pub fn sync_backup(
    exec: &dyn Exec,
    ssh_creds: &SshCredentials,
    exclude_file: &Path,
    source: &Path,
    destination: &Path,
    log_file: &Path,
) -> Result<String, SyncError> {
    // rsync -ave "ssh -l ${conf.sshUser} -i ${conf.sshIdFilename}" --compress --one-file-system --exclude-from=${conf.excludeFilename} --delete-after --delete-excluded ${conf.source} ${conf.destination} > ${conf.logFilename}
    let ssh_command = vec!["ssh", "-l", &ssh_creds.user, "-i", &ssh_creds.id_file].join(" ");
    let exclude_file = format!(
        "--exclude-from={}",
        exclude_file
            .to_str()
            .ok_or_else(|| SyncError::PathConversionError("exclude file".to_string()))?
    );
    let destination = format!(
        "{}@{}:{}",
        ssh_creds.user,
        ssh_creds.host,
        destination
            .to_str()
            .ok_or_else(|| SyncError::PathConversionError("destination".to_string()))?
    );
    let rsync_args = vec![
        "-ave",
        &ssh_command,
        "--compress",
        "--one-file-system",
        &exclude_file,
        "--delete-after",
        "--delete-excluded",
        source
            .to_str()
            .ok_or_else(|| SyncError::PathConversionError("source".to_string()))?,
        &destination,
        ">",
        log_file
            .to_str()
            .ok_or_else(|| SyncError::PathConversionError("log file".to_string()))?,
    ];

    let res = exec.exec("rsync", &rsync_args[..])?;

    Ok(res)
}

/// create a snapshot using a hard link from the backup directory to a timestamped directory in the snapshot folder
pub fn create_snapshot(
    exec: &dyn Exec,
    ssh_creds: &SshCredentials,
    backup_path: &Path,
    snapshot_path: &Path,
) -> Result<String, SyncError> {
    // cp -al "$bckPath" "$bckPath1"
    let command = "ssh";
    let args = [
        "-l",
        &ssh_creds.user,
        "-i",
        &ssh_creds.id_file,
        &ssh_creds.host,
        "cp",
        "-al",
        backup_path
            .to_str()
            .ok_or_else(|| SyncError::PathConversionError("backup".to_string()))?,
        snapshot_path
            .to_str()
            .ok_or_else(|| SyncError::PathConversionError("snapshot".to_string()))?,
    ];
    let res = exec.exec(command, &args)?;

    Ok(res)
}

/// get snapshots
pub fn get_snapshots(
    exec: &dyn Exec,
    ssh_creds: &SshCredentials,
    snapshot_path: &Path,
) -> Result<Vec<(DateTime<Utc>, String)>, SyncError> {
    // ls -A1
    Ok(exec
        .exec(
            "ssh",
            &[
                "-l",
                &ssh_creds.user,
                "-i",
                &ssh_creds.id_file,
                &ssh_creds.host,
                "ls",
                "-A1",
                snapshot_path
                    .to_str()
                    .ok_or_else(|| SyncError::PathConversionError("snapshot".to_string()))?,
            ],
        )?
        .split('\n')
        .filter_map(|s| {
            match s
                .split('_')
                .next()
                .ok_or(SyncError::SplitError)
                .and_then(|token| DateTime::parse_from_rfc3339(token).map_err(|e| e.into()))
                .map(|date| (date.into(), s))
            {
                Ok((date, s)) => Some((date, s.to_string())),
                Err(_) => None,
            }
        })
        .collect::<Vec<(DateTime<Utc>, String)>>())
}
/// review snapshots and remove the ones not complying to the policy
pub fn delete_snapshot(
    exec: &dyn Exec,
    ssh_creds: &SshCredentials,
    snapshot_path: &Path,
) -> Result<(), SyncError> {
    let snapshot_path = snapshot_path
        .to_str()
        .ok_or_else(|| SyncError::PathConversionError("snapshot".to_string()))?;

    if ["/", ""].iter().any(|&s| s == snapshot_path) {
        return Err(SyncError::PathDeletionError(snapshot_path.to_string()));
    }
    exec.exec(
        "ssh",
        &[
            "-l",
            &ssh_creds.user,
            "-i",
            &ssh_creds.id_file,
            &ssh_creds.host,
            "rm",
            "-r",
            snapshot_path,
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn sync_backup() {
        let mut mock = exec_rs::MockExec::new();

        mock.expect_exec().once().returning(|command, args| {
            assert_eq!(command, "rsync");
            assert_eq!(
                args,
                vec![
                    "-ave",
                    "ssh -l ssh_user -i ssh_id_file",
                    "--compress",
                    "--one-file-system",
                    "--exclude-from=exclude_file",
                    "--delete-after",
                    "--delete-excluded",
                    "source",
                    "ssh_user@host:destination",
                    ">",
                    "log_file",
                ]
            );
            Ok("ok".to_string())
        });

        super::sync_backup(
            &mock,
            &SshCredentials {
                user: "ssh_user".to_string(),
                id_file: "ssh_id_file".to_string(),
                host: "host".to_string(),
            },
            &Path::new("exclude_file"),
            &Path::new("source"),
            &Path::new("destination"),
            &Path::new("log_file"),
        )
        .unwrap();
    }

    #[test]
    fn create_snapshot() {
        let mut mock = exec_rs::MockExec::new();

        mock.expect_exec().once().returning(|command, args| {
            assert_eq!(command, "ssh");
            assert_eq!(
                args,
                vec![
                    "-l",
                    "ssh_user",
                    "-i",
                    "ssh_id_file",
                    "host",
                    "cp",
                    "-al",
                    "backup_path",
                    "snapshot_path"
                ]
            );
            Ok("ok".to_string())
        });

        super::create_snapshot(
            &mock,
            &SshCredentials {
                user: "ssh_user".to_string(),
                id_file: "ssh_id_file".to_string(),
                host: "host".to_string(),
            },
            &Path::new("backup_path"),
            &Path::new("snapshot_path"),
        )
        .unwrap();
    }

    #[test]
    fn get_snapshots() {
        let mut mock = exec_rs::MockExec::new();

        mock.expect_exec().once().returning(|command, args| {
            assert_eq!(command, "ssh");
            assert_eq!(
                args,
                vec![
                    "-l",
                    "ssh_user",
                    "-i",
                    "ssh_id_file",
                    "host",
                    "ls",
                    "-A1",
                    "snapshot_path"
                ]
            );
            Ok("2022-11-02T21:22:10Z_test1\n2022-11-01T21:22:10Z_test2\n".to_string())
        });

        let res = super::get_snapshots(
            &mock,
            &SshCredentials {
                user: "ssh_user".to_string(),
                id_file: "ssh_id_file".to_string(),
                host: "host".to_string(),
            },
            &Path::new("snapshot_path"),
        )
        .unwrap();
        assert_eq!(
            vec![
                (
                    Utc.ymd(2022, 11, 02).and_hms(21, 22, 10),
                    "2022-11-02T21:22:10Z_test1".to_string()
                ),
                (
                    Utc.ymd(2022, 11, 01).and_hms(21, 22, 10),
                    "2022-11-01T21:22:10Z_test2".to_string()
                )
            ],
            res
        );
    }

    #[test]
    fn delete_snapshot() {
        let mut mock = exec_rs::MockExec::new();

        mock.expect_exec().once().returning(|command, args| {
            assert_eq!(command, "ssh");
            assert_eq!(
                args,
                vec![
                    "-l",
                    "ssh_user",
                    "-i",
                    "ssh_id_file",
                    "host",
                    "rm",
                    "-r",
                    "snapshot_path"
                ]
            );
            Ok("".to_string())
        });

        super::delete_snapshot(
            &mock,
            &SshCredentials {
                user: "ssh_user".to_string(),
                id_file: "ssh_id_file".to_string(),
                host: "host".to_string(),
            },
            &Path::new("snapshot_path"),
        )
        .unwrap();
    }
}
