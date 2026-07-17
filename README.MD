What the code does:
Monitor each packet for its direction and size sent

What it does not do:
See how much data a process *actually* sent. It does not focus on how much it tried to send. I had to choose only one-this makes more sense as a network monitoring app.


Limitations: 
Cannot have uniqueness per app per interface.
Dropped Packets never appear in the result.


Possible future features:
Might try to incorprate YARA rules so that its a malware scanner (Very big target)
Still include how many bytes were sent *in total* per interface 
Include IPv6 instead of just IPv4
Include local IP (Didnt include it just because it felt too useless)
Include Country for each IP (need to do this first)

Requirements:
Linux only, only Kernel version 5.5 for x86_64, 6+ for ARM. Will not be tested on ARM. 
To check, run: 
uname -r
Also run:
ls -la /sys/kernel/btf/vmlinux
If the file doesn't exist, dont run the app.

Things you may want to change manually:
Ring Buffer Size. 4Mb is the recommended Default
SAVE_INTERVAL_SECS

What this app helps notify against:
Processes that send too much data for no reason or processes that send data that shouldn't be used at all


What this app doesn't help against:
Malware that got injected into another important process and isn't using too much data

Motivated by @Glasswire, the windows and better (for now) alternative