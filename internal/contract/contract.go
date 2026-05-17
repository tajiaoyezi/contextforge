// Package contract exposes the frozen ContextForge proto / canonical-record
// contract (task-1.1). It is the in-repo conformance surface that ties the
// frozen proto SSOT (proto/contextforge/v1/*.proto) to the Go data-plane.
//
// RED skeleton: every accessor below is a deliberate, explicit
// `panic("unimplemented: ...")` so the conformance tests fail for the right
// reason — feature absent — rather than by a compile error (s2v §2.5.1
// compiled-language RED bridge). GREEN replaces this file with the real
// implementation that parses proto/contextforge/v1/*.proto and imports the
// grpc-go generated bindings.
package contract

// SchemaVersion returns the frozen canonical-record schema version ("0.1").
func SchemaVersion() string {
	panic("unimplemented: contract.SchemaVersion — proto contract not yet frozen (task-1.1 RED)")
}

// FreezeRuleDocumented reports whether the proto documents the versioning
// freeze rule ("only add fields, never delete or renumber tags").
func FreezeRuleDocumented() bool {
	panic("unimplemented: contract.FreezeRuleDocumented — task-1.1 RED")
}

// MessageFields returns the proto field names of the given canonical message.
func MessageFields(msg string) []string {
	panic("unimplemented: contract.MessageFields — task-1.1 RED")
}

// GeneratedGoSmoke proves the Go bindings were code-generated (grpc-go, no
// FFI) by constructing a generated message and referencing the generated
// gRPC client constructor. Returns nil when codegen is wired correctly.
func GeneratedGoSmoke() error {
	panic("unimplemented: contract.GeneratedGoSmoke — task-1.1 RED")
}
