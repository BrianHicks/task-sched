pub struct Scheduler {}

impl Scheduler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn schedule(&self) {
        /*

        in a loop while we have free space in the schedule:

        1. look at the earliest bit of free time we have
        2. constrain the free time to 30 minutes
        3. if we got 30 minutes exactly, schedule a five minute break at the end and shorten the duration by five minutes
        4. select the most urgent ready task (unblocked + not waiting) at the start time
        5. if that task will take longer than the available time, deduct the time from its estimate and schedule it then
        6. if that task is shorter than the available time, schedule it as a whole, increment the start time, add it to the resolved task list for blocker calculation, and repeat from 4

        We're not too concerned if we schedule things past their due dates. The urgency algorithm should take care of most of it for us.

        Soooo I need:

        1. a way to figure out what blocks of free time are available (or at least what the *next* block of free time is)
        2. a way to annotate tasks with provisionally resolved depdendencies and partial completion
        3. a way to store scheduling decisions

        */
    }
}
