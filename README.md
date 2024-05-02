# Platform
Tested on Pop! OS 22.04

Designed to be cross platform, untested.

# Dependencies
* Run: `sudo apt install build-essential libudev-dev`

* Install Rust using Rustup (cargo 1.72.0 (103a7ff2e 2023-08-15))

# Serial Permissions
To allow a user to perform serial communication without needing sudo privileges on Ubuntu 22.04, you can add the user to the `dialout` group. This group is typically granted permissions to access serial interfaces like `/dev/ttyS0`, `/dev/ttyUSB0`, etc.

Here are the commands you'll need to run:

1. Open a terminal.

2. Add your user (in this case, "user") to the `dialout` group by using the following command:
   ```bash
   sudo usermod -a -G dialout user
   ```

3. To ensure the changes take effect, you might need to log out and log back in, or you can use the following command to apply the changes immediately:
   ```bash
   newgrp dialout
   ```

After performing these steps, the user "user" should be able to access serial communication devices without needing sudo privileges.
