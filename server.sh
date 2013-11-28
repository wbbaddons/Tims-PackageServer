#!/bin/bash
name="updatesrv"
case "$1" in
  start)
	echo "Starte updateserver"
	screen -admS $name npm start
    ;;

  stop)
	echo "Stoppe updateserver"
	screen -S $name -X quit
    ;;

  restart)
	echo "Restarte updateserver"
	$0 stop
	$0 start
	;;
	
  *)
    echo "Fehlerhafter Aufruf"
    echo "Syntax: $0 {start|stop|restart}"
    exit 1
    ;;
esac
