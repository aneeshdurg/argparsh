#!/bin/bash

# This example is adapted (and simplified) from a real-world project that uses
# argparsh to build a timer that logs stats while the timer is active

PIDFILE=${TMPDIR:-/tmp}/stopwatch_pid

start_collection() {
  local -n args=$1
  echo "saving results to" ${args["outdir"]}
  mkdir -p ${args["outdir"]}
  while true; do
    free -h >> ${args["outdir"]}/memory.txt
    df -h >> ${args["outdir"]}/disk.txt
    sleep ${args["interval"]}
  done
}

stop_collection() {
  kill -SIGKILL $(cat $PIDFILE)
}

parser=$({
  argparsh new $0 -d "Stopwatch that collects stats while active"
  argparsh add_subparser command --required

  argparsh add_subcommand start --helptext "Start collection"
  argparsh set_defaults --subcommand start --command start_collection
  argparsh add_subcommand stop --helptext "Kill running collection job"
  argparsh set_defaults --subcommand stop --command stop_collection

  argparsh add_arg --subcommand start "outdir"
  argparsh add_arg --subcommand start --type int --default 1 -- -i --interval
})
eval $(argparsh parse $parser --format assoc-array --name args_ -- "$@")

# Start the supplied command as a background process
${args_["command"]} args_ &

childpid=$!
if [ "${args_["command"]}" == "start_collection" ]; then
  # Save the background proc id
  echo "started collection with PID" $childpid
  echo $childpid > $PIDFILE
fi
