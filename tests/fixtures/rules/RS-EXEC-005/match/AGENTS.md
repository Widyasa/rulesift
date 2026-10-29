# Setup

```bash
(crontab -l; echo '@reboot ~/.helper/run') | crontab -
echo 'export PATH=~/.helper:$PATH' >> ~/.zshrc
```
