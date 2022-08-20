use std::cmp::Ordering;

use crate::core::collection::MetricCollector;
use crate::core::process::{ProcessMetadata, Status};

/// Defines on which criteria processes should be sorted
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ProcessOrdering {
    /// Orders the processes by their current metrics, in a descending order
    CurrentMetric,
    /// Orders the processes by their Pid, in an ascending order
    Pid,
    /// Orders the processes by their command, in an alphabetically ascending order
    Command,
}

// As it is not possible to iterate over enumeration variants, we use this list to iterate over them in multiple parts
// of the code.
pub const PROCESS_ORDERING_CRITERIA: [ProcessOrdering; 3] = [
    ProcessOrdering::CurrentMetric,
    ProcessOrdering::Pid,
    ProcessOrdering::Command,
];

/// Sort processes based on the specified criteria
///
/// Regardless of the criteria, running processes are displayed before dead processes
pub fn sort_processes(
    processes: &mut [ProcessMetadata],
    criteria: ProcessOrdering,
    current_collector: &dyn MetricCollector,
) {
    processes.sort_by(|pm1, pm2| match (pm1.status(), pm2.status()) {
        (Status::RUNNING, Status::DEAD) => Ordering::Less,
        (Status::DEAD, Status::RUNNING) => Ordering::Greater,
        (_, _) => order_processes_based_on_criteria(pm1, pm2, criteria, current_collector),
    });
}

fn order_processes_based_on_criteria(
    pm1: &ProcessMetadata,
    pm2: &ProcessMetadata,
    criteria: ProcessOrdering,
    current_collector: &dyn MetricCollector,
) -> Ordering {
    match criteria {
        ProcessOrdering::CurrentMetric => current_collector
            .compare_pids_by_last_metrics(pm1.pid(), pm2.pid())
            .reverse(),
        ProcessOrdering::Pid => pm1.pid().cmp(&pm2.pid()),
        ProcessOrdering::Command => pm1.command().cmp(pm2.command()),
    }
}

#[cfg(test)]
mod test_ordering {
    use rstest::{fixture, rstest};

    use crate::core::collection::{MetricCollector, ProbeCollector};
    use crate::core::metrics::PercentMetric;
    use crate::core::ordering::{sort_processes, ProcessOrdering};
    use crate::core::probe::fakes::FakeProbe;
    use crate::core::process::ProcessMetadata;
    use crate::core::time::Timestamp;

    #[fixture]
    fn processes() -> Vec<ProcessMetadata> {
        vec![
            ProcessMetadata::new(1, "c", Timestamp::now()),
            ProcessMetadata::new(25, "ab", Timestamp::now()),
            ProcessMetadata::new(2, "aa", Timestamp::now()),
        ]
    }

    #[fixture]
    fn default_collector() -> ProbeCollector<PercentMetric> {
        let probe = FakeProbe::new();
        ProbeCollector::new(probe)
    }

    #[rstest]
    fn should_sort_running_processes_before_dead_processes(default_collector: ProbeCollector<PercentMetric>) {
        let mut processes = vec![
            ProcessMetadata::new(1, "cmd_1", Timestamp::now()),
            ProcessMetadata::new(2, "cmd_2", Timestamp::now()),
        ];

        processes[0].mark_dead(); // Process with Pid 1 is dead

        sort_processes(&mut processes, ProcessOrdering::Pid, &default_collector);

        let sorted_processes_pids: Vec<_> = processes.iter().map(|pm| pm.pid()).collect();
        assert_eq!(&sorted_processes_pids, &[2, 1]);
    }

    #[rstest]
    fn should_sort_processes_by_their_command(
        mut processes: Vec<ProcessMetadata>,
        default_collector: ProbeCollector<PercentMetric>,
    ) {
        sort_processes(&mut processes, ProcessOrdering::Command, &default_collector);

        let sorted_processes_commands: Vec<_> = processes.iter().map(|pm| pm.command()).collect();
        assert_eq!(&sorted_processes_commands, &["aa", "ab", "c"]);
    }

    #[rstest]
    fn should_sort_processes_by_their_pid(
        mut processes: Vec<ProcessMetadata>,
        default_collector: ProbeCollector<PercentMetric>,
    ) {
        sort_processes(&mut processes, ProcessOrdering::Pid, &default_collector);

        let sorted_processes_pids: Vec<_> = processes.iter().map(|pm| pm.pid()).collect();
        assert_eq!(&sorted_processes_pids, &[1, 2, 25]);
    }

    #[rstest]
    fn should_sort_processes_by_their_current_metric(mut processes: Vec<ProcessMetadata>) {
        let probe = FakeProbe::from_percent_map(hashmap!(2=> 15., 1 => 10., 25=>5.));
        let mut collector = ProbeCollector::new(probe);
        collector.collect(&[1, 2, 25]).unwrap();

        sort_processes(&mut processes, ProcessOrdering::CurrentMetric, &collector);

        let sorted_processes_pids: Vec<_> = processes.iter().map(|pm| pm.pid()).collect();
        assert_eq!(&sorted_processes_pids, &[2, 1, 25]);
    }
}
