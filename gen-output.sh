#!/bin/bash
exec cargo run > "output-for-`git describe`.json"
