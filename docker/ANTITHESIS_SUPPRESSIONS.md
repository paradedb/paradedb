# Antithesis Finding Suppressions

Antithesis configures finding suppressions on its side. The specification below documents a proposed rule to request from the Antithesis team, following its [suppression workflow](https://antithesis.com/docs/product/understanding_test_results/reports/findings/#disentangle-a-known-issue). Adding this documentation does not activate a suppression, and no activation is recorded here.

## CNPG Operator Leader Election Lost

CloudNativePG intentionally exits when it loses its Kubernetes leader-election lease. API connectivity loss can prevent lease renewal and trigger this safety behavior. The operator logs `problem running manager` with error `leader election lost` before exiting with code 1. See the upstream [controller-runtime leadership-loss handler](https://github.com/kubernetes-sigs/controller-runtime/blob/v0.25.1/pkg/manager/internal.go#L581-L591) and [CNPG manager error handling](https://github.com/cloudnative-pg/cloudnative-pg/blob/v1.30.1/internal/cmd/manager/controller/controller.go).

Request the following suppression, subject to Antithesis confirming support for correlating the exit with preceding logs:

| Field         | Match                                                                                    |
| ------------- | ---------------------------------------------------------------------------------------- |
| Property      | `No unexpected container exits`                                                          |
| Namespace     | `cnpg-system`                                                                            |
| Pod           | Prefix `cnpg-cloudnative-pg-`                                                            |
| Container     | `manager`                                                                                |
| Exit code     | `1`                                                                                      |
| Preceding log | Same container instance logs `problem running manager` with error `leader election lost` |

Rationale: this specific exit is the operator's expected response to losing leadership. Keep other operator errors and all database container failures visible. Matching only the container name and exit code would also hide unrelated failures. If log correlation is unavailable, resolve the scope with Antithesis before enabling a broader rule.

Once Antithesis configures the rule, the report's **Suppressions** toggle controls whether suppressed findings are displayed. Keep recovery-liveness assertions enabled: an expected operator exit does not establish that connectivity recovered or another operator took leadership. In particular, connectivity failures during a reported fault-free window still need investigation. Preserve the operator's original exit codes and leader election behavior.
