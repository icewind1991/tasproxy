{
  dockerTools,
  tasproxy,
}:
dockerTools.buildLayeredImage {
  name = "icewind1991/tasproxy";
  tag = "latest";
  maxLayers = 5;
  contents = [
    tasproxy
    dockerTools.caCertificates
  ];
  config = {
    Cmd = ["tasproxy"];
    ExposedPorts = {
      "80/tcp" = {};
    };
  };
}
