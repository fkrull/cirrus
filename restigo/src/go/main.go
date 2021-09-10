package main

import (
    "C"
	"github.com/restic/restic/cmd/restic"
)

//export ResticMain
func ResticMain() {
	restic.Main()
}

func main() {}
