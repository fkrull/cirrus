command = "{{executable}} daemon --supervisor"
set shell = CreateObject("WScript.Shell")
shell.Run command, 0
