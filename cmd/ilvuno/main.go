// Command ilvuno is the Ilvuno command-line entry point.
//
// The M0 build supports only --version and --help. Planned commands are reported
// as not implemented rather than faked.
package main

import (
	"fmt"
	"io"
	"os"
)

// version is set at link time with -ldflags for release builds.
var version = "dev"

const helpText = `ilvuno: parallel work, verified together.

NOT USABLE YET: this build contains M0 test infrastructure only.

Usage:
  ilvuno --version    print the version and exit
  ilvuno --help       print this help and exit

Planned commands (not implemented in this build):
  init, doctor, pin, integrate, verify, show, approve, task
`

// plannedCommands are named in the roadmap but have no implementation in M0.
var plannedCommands = map[string]bool{
	"init": true, "doctor": true, "pin": true, "integrate": true,
	"verify": true, "show": true, "approve": true, "task": true,
}

func main() {
	os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
}

// run executes one invocation and returns the process exit code.
func run(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		fmt.Fprint(stdout, helpText)
		return 0
	}
	switch args[0] {
	case "--version":
		fmt.Fprintf(stdout, "ilvuno %s\n", version)
		return 0
	case "--help", "-h":
		fmt.Fprint(stdout, helpText)
		return 0
	}
	if plannedCommands[args[0]] {
		fmt.Fprintf(stderr, "ilvuno: %q is not implemented in this build\n", args[0])
		return 2
	}
	fmt.Fprintf(stderr, "ilvuno: unknown argument %q (try --help)\n", args[0])
	return 2
}
