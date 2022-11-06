use crate::{commands, config::Config, sync_error::SyncError};
use chrono::{DateTime, SecondsFormat, Utc};
use exec_rs::{CommandExec, Exec};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

pub struct Sync<T: Exec> {
    exec: T,
    config: Config,
}

impl Sync<CommandExec> {
    pub fn new(config: Config) -> Self {
        Self {
            exec: CommandExec {},
            config,
        }
    }
}

impl<T: Exec> Sync<T> {
    /// constructor
    pub fn new_with_exec(config: Config, exec: T) -> Self {
        Self { exec, config }
    }

    pub fn execute(&self) -> Result<(), SyncError> {
        self.execute_with_time(&Utc::now().into())
    }

    fn execute_with_time(&self, date_time: &DateTime<Utc>) -> Result<(), SyncError> {
        // sync backup
        log::debug!("syncing backup");
        commands::sync_backup(
            &self.exec,
            &self.config.ssh_credentials,
            Path::new(&self.config.exclude_file),
            Path::new(&self.config.source),
            Path::new(&self.config.destination),
            Path::new(&self.config.log_file),
        )?;
        // create snapshot path
        let mut snapshot_path = Path::new(&self.config.snapshot).to_path_buf();

        snapshot_path.push(format!(
            "{}_{}",
            date_time.to_rfc3339_opts(SecondsFormat::Secs, true),
            self.config.snapshot_suffix
        ));
        // create snapshot
        commands::create_snapshot(
            &self.exec,
            &self.config.ssh_credentials,
            Path::new(&self.config.destination),
            &snapshot_path,
        )?;
        // get all snapshots
        let snapshots = commands::get_snapshots(
            &self.exec,
            &self.config.ssh_credentials,
            Path::new(&self.config.snapshot),
        )?;
        // find snapshots to be deleted
        let to_be_deleted = policer::police(
            date_time,
            &self
                .config
                .policy
                .iter()
                .map(|e| e.try_into())
                .collect::<Result<Vec<chrono::Duration>, SyncError>>()?[..],
            &snapshots[..],
        );
        // remove snapshots
        for (_, delete) in to_be_deleted {
            let mut delete_path = PathBuf::from_str(&self.config.snapshot)?;

            delete_path.push(&delete);

            commands::delete_snapshot(&self.exec, &self.config.ssh_credentials, &delete_path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{custom_duration::CustomDuration, ssh_credentials::SshCredentials};
    use chrono::SecondsFormat;
    use mockall::Sequence;

    #[test]
    fn execute() {
        let mut seq = Sequence::new();
        let mut mock = exec_rs::MockExec::new();
        let date_time = Utc::now();
        let snapshot = format!(
            "snapshot/{}_test_user",
            <DateTime<Utc>>::from(date_time).to_rfc3339_opts(SecondsFormat::Secs, true)
        );

        mock.expect_exec()
            .times(1)
            .returning(|command, args| {
                assert_eq!(command, "rsync");
                assert_eq!(
                    args,
                    &[
                        "-ave",
                        "ssh -l user -i id_file",
                        "--compress",
                        "--one-file-system",
                        "--exclude-from=exclude_file",
                        "--delete-after",
                        "--delete-excluded",
                        "source",
                        "user@host:destination",
                        ">",
                        "log_file",
                    ]
                );
                Ok(String::new())
            })
            .in_sequence(&mut seq);

        mock.expect_exec()
            .times(1)
            .returning(move |command, args| {
                assert_eq!(command, "ssh");
                assert_eq!(
                    args,
                    &[
                        "-l",
                        "user",
                        "-i",
                        "id_file",
                        "host",
                        "cp",
                        "-al",
                        "destination",
                        &snapshot
                    ]
                );

                Ok(String::new())
            })
            .in_sequence(&mut seq);

        mock.expect_exec()
            .times(1)
            .returning(|command, args| {
                assert_eq!(command, "ssh");
                assert_eq!(
                    args,
                    &["-l", "user", "-i", "id_file", "host", "ls", "-A1", "snapshot",]
                );

                Ok(String::from(
                    "2022-11-01T12:00:00Z_test_user\n2022-11-01T13:00:00Z_test_user\n2022-11-01T14:00:00Z_test_user\n2022-11-01T15:00:00Z_test_user",
                ))
            })
            .in_sequence(&mut seq);

        mock.expect_exec()
            .times(1)
            .returning(|command, args| {
                assert_eq!(command, "ssh");
                assert_eq!(
                    args,
                    &[
                        "-l",
                        "user",
                        "-i",
                        "id_file",
                        "host",
                        "rm",
                        "-r",
                        "snapshot/2022-11-01T12:00:00Z_test_user",
                    ]
                );

                Ok(String::new())
            })
            .in_sequence(&mut seq);

        let config = Config {
            source: "source".to_string(),
            destination: "destination".to_string(),
            exclude_file: "exclude_file".to_string(),
            log_file: "log_file".to_string(),
            ssh_credentials: SshCredentials {
                host: "host".to_string(),
                id_file: "id_file".to_string(),
                user: "user".to_string(),
            },
            snapshot: "snapshot".to_string(),
            snapshot_suffix: "test_user".to_string(),
            policy: vec![CustomDuration::minutes(30), CustomDuration::days(2)],
        };
        let sync = Sync::new_with_exec(config, mock);

        sync.execute_with_time(&date_time.into())
            .expect("failed to execute");
    }
}
