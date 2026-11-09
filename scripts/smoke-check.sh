#!/usr/bin/env bash
# 배포 후 점검 스크립트: 공개 주소를 대상으로 확인 시나리오를 curl로 검사한다.
#
# 사용법: scripts/smoke-check.sh [공개 주소]   (기본값: https://s.zoek.workers.dev)
#
# 주의: 링크는 수정·삭제할 수 없으므로 이 스크립트가 만든 링크는 대상 서비스에
#       영구히 남는다. 한 번 실행에 6개(생성 1 + 동시 생성 5)가 생긴다.
#
# 필요한 도구: bash, curl, sed, grep, tr (jq 불필요). 하나라도 실패하면 종료 코드 1.

set -u

BASE="${1:-https://s.zoek.workers.dev}"
BASE="${BASE%/}"
RUN="$(date +%s)$$"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

PASS=0
FAIL=0

ok()   { PASS=$((PASS + 1)); echo "PASS  $1"; }
fail() { FAIL=$((FAIL + 1)); echo "FAIL  $1"; [ -n "${2:-}" ] && echo "      $2"; }

# check <설명> <기대값> <실제값>
check() {
  if [ "$2" = "$3" ]; then ok "$1"; else fail "$1" "기대: $2 / 실제: $3"; fi
}

# 상태 코드만 출력 (리디렉션을 따라가지 않음)
status() { curl -s -o /dev/null -w '%{http_code}' --max-time 30 "$@"; }

# 응답 헤더 값(소문자 이름) 추출
header() { # <헤더 파일> <이름>
  tr -d '\r' < "$1" | grep -i "^$2:" | head -n1 | sed -e "s/^[^:]*: *//"
}

# JSON 숫자/문자열/불리언 필드 추출 (평평한 JSON 전용)
json_num()  { sed -n "s/.*\"$2\":\([0-9][0-9]*\).*/\1/p" <<<"$1"; }
json_str()  { sed -n "s/.*\"$2\":\"\([^\"]*\)\".*/\1/p" <<<"$1"; }
json_bool() { grep -o "\"$2\":\(true\|false\)" <<<"$1" | head -n1 | sed -e "s/.*://"; }

# create <url> <본문 파일 접두어>: 본문은 $1.body, 상태 코드는 $1.code 에 저장
create() {
  curl -s -o "$2.body" -w '%{http_code}' --max-time 30 \
    -X POST "$BASE/api/links" -H 'Content-Type: application/json' \
    --data "{\"url\":\"$1\"}" > "$2.code"
}

# redirect_of <번호>: 상태 코드와 Location 을 "코드 Location" 으로 출력
redirect_of() {
  curl -s -o /dev/null -D "$TMP/h" --max-time 30 "$BASE/$1"
  local code; code="$(head -n1 "$TMP/h" | tr -d '\r' | awk '{print $2}')"
  echo "$code $(header "$TMP/h" location)"
}

echo "대상: $BASE"
echo "실행 ID: $RUN"
echo

# 1. 화면
curl -s -o "$TMP/index.html" -D "$TMP/h" -w '%{http_code}' --max-time 30 "$BASE/" > "$TMP/code"
check "GET / 는 200" "200" "$(cat "$TMP/code")"
case "$(header "$TMP/h" content-type)" in
  text/html*) ok "GET / 는 text/html" ;;
  *) fail "GET / 는 text/html" "Content-Type: $(header "$TMP/h" content-type)" ;;
esac

# 2. 생성
URL1="https://example.org/smoke?run=$RUN&n=1#frag"
create "$URL1" "$TMP/c1"
check "새 URL 생성은 201" "201" "$(cat "$TMP/c1.code")"
N1="$(json_num "$(cat "$TMP/c1.body")" number)"
SHORT1="$(json_str "$(cat "$TMP/c1.body")" short_url)"
case "$SHORT1" in
  "$BASE"/*) ok "short_url 이 공개 주소로 시작 ($SHORT1)" ;;
  *) fail "short_url 이 공개 주소로 시작" "short_url: $SHORT1" ;;
esac
check "새 링크는 existing:false" "false" "$(json_bool "$(cat "$TMP/c1.body")" existing)"

# 3. 이동
if [ -n "$N1" ]; then
  check "GET /$N1 는 302 이고 Location 이 원본 URL과 같음" "302 $URL1" "$(redirect_of "$N1")"
  check "단축 주소 뒤 쿼리는 무시하고 같은 원본 URL로 이동" "302 $URL1" \
    "$(curl -s -o /dev/null -D "$TMP/h" --max-time 30 "$BASE/$N1?utm=x"; echo "$(head -n1 "$TMP/h" | tr -d '\r' | awk '{print $2}') $(header "$TMP/h" location)")"
else
  fail "GET /<번호> 302 이동" "생성 응답에서 번호를 읽지 못함"
fi

# 4. 같은 URL 재입력
create "$URL1" "$TMP/c1b"
check "같은 URL 재입력은 200" "200" "$(cat "$TMP/c1b.code")"
check "같은 URL 재입력은 같은 번호" "$N1" "$(json_num "$(cat "$TMP/c1b.body")" number)"
check "같은 URL 재입력은 existing:true" "true" "$(json_bool "$(cat "$TMP/c1b.body")" existing)"

# 5. 동시 생성
for i in 1 2 3 4 5; do
  create "https://example.org/smoke?run=$RUN&concurrent=$i#frag" "$TMP/p$i" &
done
wait
NUMS=""
for i in 1 2 3 4 5; do
  code="$(cat "$TMP/p$i.code")"
  n="$(json_num "$(cat "$TMP/p$i.body")" number)"
  check "동시 생성 #$i 는 201" "201" "$code"
  NUMS="$NUMS $n"
done
check "동시 생성 5개는 서로 다른 번호" "5" "$(echo $NUMS | tr ' ' '\n' | sort -u | grep -c .)"
for i in 1 2 3 4 5; do
  n="$(json_num "$(cat "$TMP/p$i.body")" number)"
  check "동시 생성 #$i 의 /$n 은 자기 원본 URL로 302" \
    "302 https://example.org/smoke?run=$RUN&concurrent=$i#frag" "$(redirect_of "$n")"
done

# 6. 없는 번호와 잘못된 번호 형식
curl -s -o "$TMP/nf.html" -D "$TMP/h" -w '%{http_code}' --max-time 30 "$BASE/9999999999" > "$TMP/code"
check "없는 번호 /9999999999 는 404" "404" "$(cat "$TMP/code")"
case "$(header "$TMP/h" content-type)" in
  text/html*) ok "404 응답은 text/html" ;;
  *) fail "404 응답은 text/html" "Content-Type: $(header "$TMP/h" content-type)" ;;
esac
check "/042 는 404" "404" "$(status "$BASE/042")"
check "/0 은 404" "404" "$(status "$BASE/0")"

# 7. 변경 요청은 405, 링크는 그대로
if [ -n "$N1" ]; then
  for m in PUT PATCH DELETE; do
    check "$m /$N1 는 405" "405" "$(status -X "$m" "$BASE/$N1")"
  done
  check "변경 요청 뒤에도 /$N1 은 같은 원본 URL로 302" "302 $URL1" "$(redirect_of "$N1")"
fi

# 8. 잘못된 URL
create "ftp://example.org" "$TMP/b1";                    check "ftp:// URL 은 400" "400" "$(cat "$TMP/b1.code")"
create "https://user:pass@example.org" "$TMP/b2";        check "사용자 정보가 있는 URL 은 400" "400" "$(cat "$TMP/b2.code")"
create "$BASE/$N1" "$TMP/b3";                            check "이 서비스의 단축 주소(자기 링크)는 400" "400" "$(cat "$TMP/b3.code")"

# 9. 알 수 없는 필드
code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 30 -X POST "$BASE/api/links" \
  -H 'Content-Type: application/json' \
  --data "{\"url\":\"https://example.org/smoke?run=$RUN&extra=1\",\"number\":1}")"
check "url 외 필드가 있는 본문은 400" "400" "$code"

echo
echo "결과: PASS $PASS, FAIL $FAIL"
[ "$FAIL" -eq 0 ]
