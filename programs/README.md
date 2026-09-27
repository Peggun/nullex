#Nullex Programs
This `programs` folder holds the default built in apps for the kernel. 
Currently, it contains:<br>
`nush` - Nullex shell environment.<br>
`nget` - Nullex web retrival tool and package manager.<br>

These are currently the only two files. The reason being:<br>
a) it gets complicated directly embedding RamFS files currently<br>
b) you need a shell environment, and<br>
c) you are able to download other programs using `nget`.

If you would like to add a program into the programs folder, please create an [issue](https://github.com/Peggun/nullex/issues) and explain your program(s) and why they should be embedded within the kernel. If otherwise, you will be able to your program into the [nullex-packages](https://peggun.github.io/nullex-packages) github repository, where programs can be stored there. Eventually (hopefully) this will become a small programs repository, and eventually then becomes a fully fledged server.