use chrono::{DateTime, FixedOffset};
use exec_rs::{CommandExec, Exec};

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error(transparent)]
    ExecError(#[from] exec_rs::ExecError),
    #[error("split error")]
    SplitError,
    #[error("path deletion error ({0})")]
    PathDeletionError(String),
    #[error(transparent)]
    ChronoParseError(#[from] chrono::ParseError),
}

pub struct Sync<T: Exec> {
    exec: T,
}

pub struct SshCredentials {
    user: String,
    id_file: String,
    host: String,
}

impl Default for Sync<CommandExec> {
    fn default() -> Self {
        Self {
            exec: CommandExec {},
        }
    }
}

impl<T: Exec> Sync<T> {
    /// constructor
    pub fn new(exec: T) -> Self {
        Self { exec }
    }

    /// run rsync to synchronize the local files with the files on the server
    pub fn sync_backup(
        &self,
        ssh_creds: &SshCredentials,
        exclude_file: &str,
        source: &str,
        destination: &str,
        log_file: &str,
    ) -> Result<String, SyncError> {
        // rsync -ave "ssh -l ${conf.sshUser} -i ${conf.sshIdFilename}" --compress --one-file-system --exclude-from=${conf.excludeFilename} --delete-after --delete-excluded ${conf.source} ${conf.destination} > ${conf.logFilename}
        let ssh_command = vec!["ssh", "-l", &ssh_creds.user, "-i", &ssh_creds.id_file].join(" ");
        let exclude_file = format!("--exclude-from={}", exclude_file);
        let rsync_args = vec![
            "-ave",
            &ssh_command,
            "--compress",
            "--one-file-system",
            &exclude_file,
            "--delete-after",
            "--delete-excluded",
            source,
            destination,
            ">",
            log_file,
        ];

        let res = self.exec.exec("rsync", &rsync_args[..])?;

        Ok(res)
    }

    /// create a snapshot using a hard link from the backup directory to a timestamped directory in the snapshot folder
    pub fn create_snapshot(
        &self,
        ssh_creds: &SshCredentials,
        backup_path: &str,
        snapshot_path: &str,
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
            backup_path,
            snapshot_path,
        ];
        let res = self.exec.exec(command, &args)?;

        Ok(res)
    }

    /// get snapshots
    pub fn get_snapshots(
        &self,
        ssh_creds: &SshCredentials,
        snapshot_path: &str,
    ) -> Result<Vec<(DateTime<FixedOffset>, String)>, SyncError> {
        // ls -A1
        Ok(self
            .exec
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
                    snapshot_path,
                ],
            )?
            .split('\n')
            .filter_map(|s| {
                match s
                    .split('_')
                    .nth(0)
                    .ok_or(SyncError::SplitError)
                    .and_then(|token| DateTime::parse_from_rfc3339(token).map_err(|e| e.into()))
                    .and_then(|date| Ok((date, s)))
                {
                    Ok((date, s)) => Some((date, s.to_string())),
                    Err(_) => None,
                }
            })
            .collect::<Vec<(DateTime<FixedOffset>, String)>>())
    }
    /// review snapshots and remove the ones not complying to the policy
    pub fn delete_snapshot(
        &self,
        ssh_creds: &SshCredentials,
        snapshot_path: &str,
    ) -> Result<(), SyncError> {
        if ["/", ""].iter().any(|&s| s == snapshot_path) {
            return Err(SyncError::PathDeletionError(snapshot_path.to_string()));
        }
        self.exec.exec(
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
}

#[cfg(test)]
mod test {
    use chrono::TimeZone;

    use super::*;

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
                    "destination",
                    ">",
                    "log_file",
                ]
            );
            Ok("ok".to_string())
        });

        let sync = Sync::new(mock);

        sync.sync_backup(
            &SshCredentials {
                user: "ssh_user".to_string(),
                id_file: "ssh_id_file".to_string(),
                host: "host".to_string(),
            },
            "exclude_file",
            "source",
            "destination",
            "log_file",
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

        let sync = Sync::new(mock);

        sync.create_snapshot(
            &SshCredentials {
                user: "ssh_user".to_string(),
                id_file: "ssh_id_file".to_string(),
                host: "host".to_string(),
            },
            "backup_path",
            "snapshot_path",
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
            Ok("2022-11-02T21:22:10+01:00_test1\n2022-11-01T21:22:10+01:00_test2\n".to_string())
        });

        let sync = Sync::new(mock);

        let res = sync
            .get_snapshots(
                &SshCredentials {
                    user: "ssh_user".to_string(),
                    id_file: "ssh_id_file".to_string(),
                    host: "host".to_string(),
                },
                "snapshot_path",
            )
            .unwrap();
        assert_eq!(
            vec![
                (
                    FixedOffset::east(3600)
                        .ymd(2022, 11, 02)
                        .and_hms(21, 22, 10),
                    "2022-11-02T21:22:10+01:00_test1".to_string()
                ),
                (
                    FixedOffset::east(3600)
                        .ymd(2022, 11, 01)
                        .and_hms(21, 22, 10),
                    "2022-11-01T21:22:10+01:00_test2".to_string()
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

        let sync = Sync::new(mock);

        sync.delete_snapshot(
            &SshCredentials {
                user: "ssh_user".to_string(),
                id_file: "ssh_id_file".to_string(),
                host: "host".to_string(),
            },
            "snapshot_path",
        )
        .unwrap();
    }
}
