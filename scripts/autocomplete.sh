#!/usr/bin/env bash

BUFFER="$1"

IFS=' ' read -r -a WORDS <<< "$BUFFER"
WORD_COUNT=${#WORDS[@]}
CURRENT_WORD="${WORDS[$((WORD_COUNT - 1))]}"

if [ "$WORD_COUNT" -eq 1 ]; then
    # command completion
    COMPREPLY=($(compgen -c -- "$CURRENT_WORD"))
else
    # file completion
    COMPREPLY=($(compgen -f -- "$CURRENT_WORD"))
fi


if [ "${#COMPREPLY[@]}" -eq 1 ]; then
    echo "${COMPREPLY[0]#$CURRENT_WORD}"
    exit 0
fi

printf '%s\n' "${COMPREPLY[@]}"