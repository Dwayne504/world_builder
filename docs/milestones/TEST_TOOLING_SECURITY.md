# Test tooling dependency update

Dependency installation on 7 October 2026 reported advisories affecting Vitest,
its mocker and worker tooling, and source-map-js. The test runner is now on the
patched Vitest 4.1 line; source-map-js is updated within its existing compatible
range. No product behavior or Rust dependency changed.

Validation on Windows: typecheck, lint, format check and production build passed.
All 332 frontend tests passed on Vitest 4.1.11 with four workers. The initial full
run had one timeout while multiple builds were active; the complete bounded-worker
rerun passed without increasing test timeouts or changing assertions. Sandbox file
resolution restrictions required running tests and the build outside that sandbox.
The production build retains its existing bundle-size advisory.

`npm audit` reports zero known vulnerabilities in the resulting dependency graph
at verification time. This result is time-specific and is not a guarantee against
future advisories. Rust suites were not rerun for this dependency-only change;
the Windows build workload separately exercises the unchanged Rust baseline.
