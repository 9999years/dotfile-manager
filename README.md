# dotfile-manager

dotfile-manager is a program to provide more sophisticated and reliable
managment of dotfiles across multiple computers.

In particular, it will support

- Linking files to specific locations depending on various conditions
  - In particular, depending on operating system, hostname, etc.
- Discovering newly-created dotfiles and determining if they should be
  managed or not
- Only _partially_ syncing dotfiles. In particular, we may want to _merge_
  some partial configuration with some larger auto-generated configuration.
  We also might want to do this conditionally, or add templating.
- Adding patches to dotfiles, again conditionally.
