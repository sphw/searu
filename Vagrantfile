# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  config.vm.box = "nixbox64"
  config.vm.provider :libvirt do |libvirt|
    # Enable KVM nested virtualization
    libvirt.nested = true
    libvirt.cpu_mode = "host-model"
  end
  #config.vm.synced_folder ".", type: "rsync", rsync__exclude: [".git/", "target/", "searu-node/blobs"]
end
