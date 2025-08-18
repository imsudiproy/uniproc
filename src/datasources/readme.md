## Total Jiffies for all CPU(s) and individual CPU can be fetched from `/proc/stat`
```
$ cat /proc/stat | grep cpu

# CPU user nice system idle iowait irq softirq steal guest guest_nice

cpu 365430 546 105481 2862563 1252 0 10568 0 0 0
cpu0 47076 48 15238 743800 329 0 2116 0 0 0
cpu1 43550 43 14460 303602 26 0 1584 0 0 0
cpu2 46679 39 12309 302736 21 0 1053 0 0 0
cpu3 46041 72 12112 303436 15 0 636 0 0 0
cpu4 46400 90 13997 300582 568 0 661 0 0 0
cpu5 46818 16 13045 301880 188 0 2931 0 0 0
cpu6 44507 31 12307 302683 64 0 1213 0 0 0
cpu7 44356 58 12010 303840 38 0 370 0 0 0
```

## iffies for a particular process can be fetched from `/proc/{pid}/stat`
```
cat /proc/3465/stat | awk '{print $14, $15, $16, $17}'

# User System User(Waited for Children) System(Waited for Children)

59175 13926 52717 10318
```
For eg: If Total CPU time (User + System) across 8 cores is 450 Jiffies (sampled per second) and Process CPU time (User + System + User of Children + System of Children) is 50 Jiffies (sampled per second), then utilization in percentage can be calculated as follows:
```
# Percentage = (100 * Process Jiffies)/Total CPU Jiffies

(100 * 50)/450 = 11.111%

```
Ref: https://www.anshulpatel.in/posts/linux_cpu_percentage/