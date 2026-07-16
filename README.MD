What the code does:
Monitor each packet for its direction and size sent

What it does not do:
See how much a packet *intended* to send. Only actual size left from the device

Limitations: 
Cannot have uniqueness per app per interface.
Dropped Packets never appear in the result.

Possible future features:
Might try to incorprate YARA rules so that its a malware scanner (Very big target)
Still include how many bytes were sent *in total* per interface 

Requirements:
Linux only, only Kernel version 5.5 for x86_64, 6+ for ARM. Will not be tested on ARM. 

Things you may want to change manually:
Ring Buffer Size. 4Mb is the recommended Default