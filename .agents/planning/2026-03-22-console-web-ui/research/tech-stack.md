# Tech Stack Research

## Chosen Stack
- Frontend: Svelte 5 (runes) + TypeScript
- Build: Vite 7
- Styling: Tailwind v4 + shadcn-svelte
- Editor: CodeMirror 6
- Backend: Axum (Rust) REST API
- Assets: Hybrid (Vite dev server in dev, rust-embed in prod)

## Ralph Orchestrator's Stack (for reference)
- React 19 + TypeScript + Vite 7 + Tailwind v4 + shadcn/ui
- Zustand + TanStack React Query
- @xyflow/react for visual workflow builder
- Axum (Rust) JSON-RPC API + WebSocket
- Assets served separately (not embedded)

## Key Svelte 5 Adopters
NYT, IKEA, 1Password, Decathlon, Chess.com, Square, NBA, Brave, Avast

## API Decision
REST chosen over JSON-RPC for simplicity and natural mapping to file/resource model.
WebSocket can be added later for live features.
