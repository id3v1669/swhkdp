#!/usr/bin/env bash

kill $(pidof swhks)

swhks & pkexec swhkd
