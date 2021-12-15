use crate::core::process::{Pid, ProcessMetadata};

#[derive(Default)]
pub struct ProcessSelector {
    sorted_processes: Vec<ProcessMetadata>,
    // We have to track the selected process using its Pid and not its index, as the position of the selected process
    // might change in sorted_processes over time
    selected_pid: Option<Pid>,
}

impl ProcessSelector {
    pub fn set_processes(&mut self, processes: &[ProcessMetadata]) {
        self.sorted_processes = processes.into();
    }

    pub fn selected_process(&self) -> Option<&ProcessMetadata> {
        self.selected_index().map(|idx| self.sorted_processes.get(idx).unwrap())
    }

    pub fn next_process(&mut self) {
        let next_index = self
            .selected_index()
            .map(|idx| idx + 1)
            .map(|next_idx| next_idx.min(self.sorted_processes.len().saturating_sub(1)));
        self.set_selected_process_from_index(next_index);
    }

    pub fn previous_process(&mut self) {
        let prev_index = self.selected_index().map(|idx| idx.saturating_sub(1));
        self.set_selected_process_from_index(prev_index);
    }

    // If it returns Some(idx), idx is always within bound of self.sorted_process
    fn selected_index(&self) -> Option<usize> {
        self.selected_pid
            .and_then(|pid| self.find_index_of_process(pid))
            .or_else(|| self.sorted_processes.get(0).and(Some(0)))
    }

    fn find_index_of_process(&self, pid: Pid) -> Option<usize> {
        self.sorted_processes.iter().position(|pm| pm.pid() == pid)
    }

    fn set_selected_process_from_index(&mut self, index: Option<usize>) {
        self.selected_pid = match index {
            None => None,
            Some(idx) => Some(self.sorted_processes.get(idx).unwrap().pid()),
        }
    }
}

#[cfg(test)]
mod test_processes {
    use rstest::*;

    use crate::core::process::ProcessMetadata;
    use crate::ctrl::processes::ProcessSelector;

    #[fixture]
    fn processes() -> Vec<ProcessMetadata> {
        vec![
            ProcessMetadata::new(1, "cmd_1"),
            ProcessMetadata::new(2, "cmd_2"),
            ProcessMetadata::new(3, "cmd_3"),
        ]
    }

    #[rstest]
    fn test_should_have_no_selected_process_when_no_process_defined() {
        let selector = ProcessSelector::default();

        assert_eq!(selector.selected_process(), None);
    }

    #[rstest]
    fn test_should_select_first_process_by_default(processes: Vec<ProcessMetadata>) {
        let mut selector = ProcessSelector::default();
        selector.set_processes(processes.as_slice());

        assert_eq!(selector.selected_process(), Some(&processes[0]));
    }

    #[rstest]
    fn test_should_select_next_process(processes: Vec<ProcessMetadata>) {
        let mut selector = ProcessSelector::default();
        selector.set_processes(processes.as_slice());
        selector.next_process();

        assert_eq!(selector.selected_process(), Some(&processes[1]));
    }

    #[rstest]
    fn test_should_select_previous_process(processes: Vec<ProcessMetadata>) {
        let mut selector = ProcessSelector::default();
        selector.set_processes(processes.as_slice());
        selector.next_process();
        selector.previous_process();

        assert_eq!(selector.selected_process(), Some(&processes[0]));
    }

    #[rstest]
    fn test_should_lock_at_first_process(processes: Vec<ProcessMetadata>) {
        let mut selector = ProcessSelector::default();
        selector.set_processes(processes.as_slice());
        selector.previous_process();

        assert_eq!(selector.selected_process(), Some(&processes[0]));
    }

    #[rstest]
    fn test_should_lock_at_last_process(processes: Vec<ProcessMetadata>) {
        let mut selector = ProcessSelector::default();
        selector.set_processes(processes.as_slice());
        (0..10).for_each(|_| selector.next_process());

        assert_eq!(selector.selected_process(), Some(processes.last().unwrap()));
    }
}
