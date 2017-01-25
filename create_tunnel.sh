 autossh -M 10984 -N -f -o "PubkeyAuthentication=yes" -o "PasswordAuthentication=no" -L 12400:localhost:12400 ubuntu@sereca-maas.cloudandheat.com -p 10104
