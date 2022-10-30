use exec_rs::{CommandExec, Exec};

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error(transparent)]
    ExecError(#[from] exec_rs::ExecError),
}

pub struct Sync<T: Exec> {
    exec: T,
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
        ssh_user: &str,
        ssh_id_file: &str,
        exclude_file: &str,
        source: &str,
        destination: &str,
        log_file: &str,
    ) -> Result<String, SyncError> {
        // rsync -ave "ssh -l ${conf.sshUser} -i ${conf.sshIdFilename}" --compress --one-file-system --exclude-from=${conf.excludeFilename} --delete-after --delete-excluded ${conf.source} ${conf.destination} > ${conf.logFilename}
        let ssh_command = vec!["ssh", "-l", ssh_user, "-i", ssh_id_file].join(" ");
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
        ssh_user: &str,
        ssh_id_file: &str,
        host: &str,
        backup_path: &str,
        snapshot_path: &str,
    ) -> Result<String, SyncError> {
        // cp -al "$bckPath" "$bckPath1"
        let command = "ssh";
        let args = [
            "-l",
            ssh_user,
            "-i",
            ssh_id_file,
            host,
            "cp",
            "-al",
            backup_path,
            snapshot_path,
        ];
        let res = self.exec.exec(command, &args)?;

        Ok(res)
    }

    /// review snapshots and remove the ones not complying to the policy
    pub fn police_snapshots(&self) -> Result<(), SyncError> {
        todo!()
    }
}

#[cfg(test)]
mod test {
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
            "ssh_user",
            "ssh_id_file",
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
            "ssh_user",
            "ssh_id_file",
            "host",
            "backup_path",
            "snapshot_path",
        )
        .unwrap();
    }
}
