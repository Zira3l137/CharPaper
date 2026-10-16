use std::thread;

use sysinfo::Pid;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;

// The app's own share of the CPU and its memory. Only this process is ever looked up, which
// takes well under a millisecond.
pub(crate) struct Process {
    system: System,
    pid: Option<Pid>,
    cores: usize,
}

pub(crate) struct Usage {
    pub cpu_percent: Option<f32>,
    pub memory_mb: Option<f32>,
}

impl Process {
    pub(crate) fn new() -> Self {
        let mut process = Self {
            system: System::new(),
            pid: sysinfo::get_current_pid().ok(),
            cores: thread::available_parallelism().map_or(1, usize::from),
        };
        // CPU use is worked out between two looks, so the first one only starts the clock.
        process.refresh();
        process
    }

    pub(crate) fn cores(&self) -> usize {
        self.cores
    }

    pub(crate) fn sample(&mut self) -> Usage {
        self.refresh();
        let process = self.pid.and_then(|pid| self.system.process(pid));
        Usage {
            // sysinfo counts one busy core as 100%.
            cpu_percent: process.map(|p| p.cpu_usage() / self.cores as f32),
            memory_mb: process.map(|p| p.memory() as f32 / (1024.0 * 1024.0)),
        }
    }

    fn refresh(&mut self) {
        let Some(pid) = self.pid else {
            return;
        };
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
    }
}
