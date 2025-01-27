# task-sched

`task-sched` reads your calendar availability from [cal.com](https://cal.com) and combines it with your [Taskwarrior](https://taskwarrior.org/) database to make a schedule.

To do this, it uses a UDA, `estimate`, and combines that with the normal urgency metrics to find the best task to recommend at any given time.

Age and due dates are calculated at the time when the tasks are scheduled. We use the same urgency calculation as in Taskwarrior, but pretend that task without a due date are due about a month after they're added so that work without due dates can be scheduled before work with far-away due dates, absent other factors.

If a task has the `+meta` tag, it will be treated as a "stop and add next steps or complete this task" signal (about 10 minutes.)

The behavior of this program is fairly custom to me. If someone else wants to use it, please let me know and I can try to add more/different CLI flags to turn some of that down.
