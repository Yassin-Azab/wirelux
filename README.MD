What the code does:
Monitor each packet for its direction and size sent
See how much data a process *actually* sent.

What it does not do:
It does not focus on how much it tried to send. I had to choose only one-this makes more sense as a network monitoring app.


Limitations: 
Cannot have uniqueness per app per interface.
Dropped Packets never appear anywhere to avoid the eBPF crashing.
If a UDP buffer is smaller than the incoming message, the application will record the intended message size, and not the size put into the buffer. Will fix it another day.
By far the biggest limitation I found during testing. There is no efficient method of getting a process name accurately, so I decided to use /proc/{pid}/comm, but the risk of a PID being used by a different process a fraction of a second later. If they PID gets disposed (and unused by a different process) the Kernel thread name is used as a fallback, may be something like "Connection Thread"


Possible future features:
Might try to incorprate YARA rules so that its a malware scanner (Very big target)
Still include how many bytes were sent *in total* per interface 
Include IPv6 instead of just IPv4
Include local IP (Didnt include it just because it felt too useless)
Include Country for each IP (need to do this first)

Requirements:
Linux only, only Kernel version 5.11 for x86_64, 6+ for ARM. Will not be tested on ARM, will make a guide on how to build in ARM if there is demand.
To check Kernel version, run: 
uname -r
Also run:
ls -la /sys/kernel/btf/vmlinux
If the file doesn't exist, dont run the app.

Things you may want to change manually:
Ring Buffer Size. 4Mb is the recommended Default
SAVE_INTERVAL_SECS. How often to save the data to the database. Default is 30. 

What this app helps notify against:
Processes that send too much data for no reason or processes that send data that shouldn't be used at all


What this app doesn't help against:
Malware that got injected into another important process and isn't using too much data

Motivated by @Glasswire, the windows and better (for now) alternative