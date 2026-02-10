## ThrottleStop details
Creation of this tool is highly inspired by [this research](https://securelist.com/av-killer-exploiting-throttlestop-sys/117026/) from Kaspersky. In this article, they described and AV Killing software, which uses the vulnerable ThrottleStop.sys to get arbitrary memory read and write, to patch one of the kernel functions and cause a process termination.

To translate virtual memory addresses to physical, attackers used open source library [superfetch](https://github.com/jonomango/superfetch/tree/main). I thought it will be interesting to implement, and I created [my own rust crate](https://github.com/I3r1h0n/SuperFetch). See the repository readme and linked articles for details.

I've also run a little bit of a research, to find a function that will allow me to kill processes. And I stumbled across this call in NtTerminateProcess:
<div align="center"><img width="500" src="../assets/psp.png"></div><br>

This function is also called from another location - PsTerminateProcess. `PsTerminateProcess` takes only two arguments - `PEPROCESS` and `NTSTATUS` (for process exit code):
<div align="center"><img width="500" src="../assets/ps.png"></div><br>

That's exactly what I needed. But the PsTerminateProcess isn't exported from ntoskrnl.exe. By following the cross references, I found this function:
<div align="center"><img width="500" src="../assets/whea.png"></div><br>

It's exported by `ntoskrnl.exe`, and calls the PsTerminateProcess! This means that we can easily find the offset to the `WheaTerminateProcess`, and, using our read primitive - find the call instruction and calculate the address of `PsTerminateProcess`.

After this, our exploit logic becomes pretty straight forward:
1. Obtain RW primitive
2. Setup SuperFetch
3. Get base address of `ntoskrnl`
4. Get address of the function that we will patch
5. Find address of `WheaTerminateProcess`
6. Find system `_EPROCESS`
7. Using read primitive - find address of `PsTerminateProcess`
8. Iterate over ActiveProcessList and find target process
9. Patch the target function to call the `PsTerminateProcess`
10. Call the target function syscall wrapper from `ntdll.dll`
11. Restore original target function stub

To see my implementation of this in rust, see the `/sigurd/src/driver/throttlestop/mod.rs` file.