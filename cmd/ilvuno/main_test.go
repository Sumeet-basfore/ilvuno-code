package main

import (
	"bytes"
	"strings"
	"testing"
)

func TestVersionPrintsDevAndExitsZero(t *testing.T) {
	// Arrange
	var out, errOut bytes.Buffer

	// Act
	code := run([]string{"--version"}, &out, &errOut)

	// Assert
	if code != 0 {
		t.Fatalf("exit = %d, want 0 (stderr %q)", code, errOut.String())
	}
	if got, want := out.String(), "ilvuno dev\n"; got != want {
		t.Fatalf("stdout = %q, want %q", got, want)
	}
}

func TestHelpStatesNotUsableYetAndListsPlannedCommands(t *testing.T) {
	var out bytes.Buffer

	code := run([]string{"--help"}, &out, &bytes.Buffer{})

	if code != 0 {
		t.Fatalf("exit = %d, want 0", code)
	}
	for _, want := range []string{"NOT USABLE YET", "not implemented in this build", "integrate"} {
		if !strings.Contains(out.String(), want) {
			t.Fatalf("help missing %q:\n%s", want, out.String())
		}
	}
}

func TestPlannedCommandIsReportedNotImplemented(t *testing.T) {
	var out, errOut bytes.Buffer

	code := run([]string{"integrate"}, &out, &errOut)

	if code != 2 {
		t.Fatalf("exit = %d, want 2", code)
	}
	if !strings.Contains(errOut.String(), "not implemented in this build") {
		t.Fatalf("stderr = %q, want a not-implemented message", errOut.String())
	}
	if out.Len() != 0 {
		t.Fatalf("stdout = %q, want empty", out.String())
	}
}

func TestUnknownArgumentExitsTwo(t *testing.T) {
	var errOut bytes.Buffer

	code := run([]string{"--bogus"}, &bytes.Buffer{}, &errOut)

	if code != 2 {
		t.Fatalf("exit = %d, want 2", code)
	}
	if !strings.Contains(errOut.String(), "unknown argument") {
		t.Fatalf("stderr = %q", errOut.String())
	}
}
