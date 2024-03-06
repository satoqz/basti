#!/usr/bin/env python3

from os import path
import masoud

PROJECT_ROOT = path.dirname(__file__)


class Etcd(masoud.Service):
    def name(self) -> str:
        return "etcd"

    def container_name(self) -> str:
        return "basti-etcd"

    def image(self) -> str:
        return "satoqz.net/etcd:latest"

    def dockerfile(self) -> tuple[str, str]:
        return (path.join(PROJECT_ROOT, "docker/etcd.Dockerfile"), PROJECT_ROOT)

    DEFAULT_CLIENT_PORT = 2379
    DEFAULT_PEER_PORT = 2380

    def ports(self) -> list[tuple[int, int]]:
        client_port = (
            self.host.get_var("etcd_client_port", int) or self.DEFAULT_CLIENT_PORT
        )
        peer_port = self.host.get_var("etcd_peer_port", int) or self.DEFAULT_PEER_PORT
        return [
            (client_port, self.DEFAULT_CLIENT_PORT),
            (peer_port, self.DEFAULT_PEER_PORT),
        ]

    DEFAULT_VOLUME_NAME = "example-etcd-data"
    VOLUME_MOUNT = "/data"

    def volumes(self) -> list[tuple[str, str]]:
        volume_name = self.host.get_var("etcd_volume", str) or self.DEFAULT_VOLUME_NAME
        return [(volume_name, self.VOLUME_MOUNT)]

    DEFAULT_INITIAL_CLUSTER_TOKEN = "example-etcd-cluster"

    def command(self) -> list[str]:
        ip = self.host.must_get_var("ip", str)
        client_port = (
            self.host.get_var("etcd_client_port", int) or self.DEFAULT_CLIENT_PORT
        )

        initial_cluster_token = (
            self.host.get_var("etcd_initial_cluster_token", str)
            or self.DEFAULT_INITIAL_CLUSTER_TOKEN
        )

        etcd_cluster_string = ",".join(
            f"{host.name}=http://{host.must_get_var("ip", str)}:{host.get_var("etcd_peer_port") or self.DEFAULT_PEER_PORT}"
            for host in self.group.get_hosts()
        )

        return [
            "etcd",
            f"--name={self.host.name}",
            f"--data-dir={self.VOLUME_MOUNT}/data",
            f"--wal-dir={self.VOLUME_MOUNT}/wal",
            f"--initial-advertise-peer-urls=http://{ip}:{self.DEFAULT_PEER_PORT}",
            f"--listen-peer-urls=http://0.0.0.0:{self.DEFAULT_PEER_PORT}",
            f"--listen-client-urls=http://0.0.0.0:{self.DEFAULT_CLIENT_PORT}",
            f"--advertise-client-urls=http://{ip}:{client_port}",
            f"--initial-cluster-token={initial_cluster_token}",
            f"--initial-cluster={etcd_cluster_string}",
            "--initial-cluster-state=new",
        ]


class Bastid(masoud.Service):
    def name(self) -> str:
        return "bastid"

    def image(self) -> str:
        return "satoqz.net/bastid:latest"

    def dockerfile(self) -> tuple[str, str]:
        return (path.join(PROJECT_ROOT, "docker/bastid.Dockerfile"), PROJECT_ROOT)

    DEFAULT_PORT = 1337

    def ports(self) -> list[tuple[int, int]]:
        port = self.host.get_var("bastid_port", int) or self.DEFAULT_PORT
        return [(port, self.DEFAULT_PORT)]

    DEFAULT_WORKERS = 1

    def command(self) -> list[str]:
        workers = self.host.get_var("bastid_workers", int) or self.DEFAULT_WORKERS
        etcd_cluster = [
            f"http://{host.must_get_var("ip", str)}:{host.get_var("etcd_client_port", int) or Etcd.DEFAULT_CLIENT_PORT}"
            for host in self.group.get_hosts()
        ]
        return [
            "bastid",
            f"--name={self.host.name}",
            f"--workers={workers}",
            f"--etcd={",".join(etcd_cluster)}",
            f"--listen=0.0.0.0:{self.host.get_var("bastid_port", int) or self.DEFAULT_PORT}",
        ]


if __name__ == "__main__":
    masoud.cli(services=[Etcd(), Bastid()])
