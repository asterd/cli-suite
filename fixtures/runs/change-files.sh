#!/bin/sh
set -eu
printf 'created\n' > created.txt
printf 'modified\n' > modified.txt
rm -f deleted.txt
