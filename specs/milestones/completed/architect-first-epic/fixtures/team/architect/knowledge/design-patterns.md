# Design Patterns

Prefer composition over inheritance in Go designs. Go does not have classical
inheritance — use struct embedding and interfaces to achieve polymorphism.

## Composition Pattern

```go
type HealthChecker struct {
    client    client.Client
    recorder  record.EventRecorder
}

type Reconciler struct {
    HealthChecker           // Embedded — composition, not inheritance
    scheme *runtime.Scheme
}
```

## Interface-Based Design

Define narrow interfaces at the consumer site. Accept interfaces, return
concrete types.

```go
type StatusReporter interface {
    ReportStatus(ctx context.Context) (Status, error)
}
```

## Guidelines

1. Prefer composition — embed structs instead of extending base classes
2. Keep interfaces small (1-3 methods)
3. Define interfaces where they are used, not where they are implemented
4. Use functional options for complex constructors
