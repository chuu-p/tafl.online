{...}: {
  services.tempo = {
    enable = true;
    settings = {
      target = "all";
      server = {
        http_listen_port = 3200;
        grpc_listen_port = 9095;
      };
      storage = {
        trace = {
          backend = "local";
          local.path = "/var/lib/tempo/traces";
          wal.path = "/var/lib/tempo/wal";
        };
      };
      distributor.receivers = {
        otlp.protocols = {
          http.endpoint = "127.0.0.1:4318";
          grpc.endpoint = "127.0.0.1:4317";
        };
          zipkin.endpoint = "127.0.0.1:9411";
          jaeger.protocols.thrift_http.endpoint = "127.0.0.1:14268";
      };
    };
  };

  systemd.services.tempo.serviceConfig.StateDirectory = "tempo";
}
