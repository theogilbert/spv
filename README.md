# spv

[![CI](https://github.com/theogilbert/spv/actions/workflows/build.yml/badge.svg)](https://github.com/theogilbert/spv/actions/workflows/build.yml)

Spv is a terminal-based tool to monitor running processes.

![](doc/images/spv.gif)

Currently, the following process metrics can be monitored:

- CPU usage
- Network bandwidth

Additional metrics should be supported in the future.

## Collecting network bandwidth metrics

When building spv without additional parameters, spv will only collect CPU usage metrics.

A feature flag needs to be specified to also collect network bandwidth metrics:

```shell
cargo build --features netio
```

To allow the produced executable to collect metrics, additional permissions need to be set to it:

```shell
sudo setcap cap_net_raw,cap_net_admin=eip path/to/spv
```

Finally, the executable must be executed with elevated permissions:

```shell
sudo path/to/spv
```
