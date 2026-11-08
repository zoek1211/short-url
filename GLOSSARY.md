# short-url

로그인 없이 긴 주소를 숫자 하나로 줄여 주는 단축 주소 서비스.

## Language

**원본 URL**:
사용자가 줄이려고 입력한 `http` 또는 `https` 주소.
_Avoid_: 긴 주소, 대상 주소, target URL

**번호**:
링크 하나를 가리키는, 0으로 시작하지 않는 양의 정수. 한 번 발급되면 다른 원본 URL에 다시 쓰이지 않는다.
_Avoid_: ID, short code, slug, 별칭

**단축 주소**:
서비스의 기본 주소 뒤에 번호를 붙인 전체 주소. 예: `https://host/42`.
_Avoid_: 짧은 URL, short URL

**링크**:
원본 URL 하나와 번호 하나의 영구적인 짝. 만든 뒤에는 바뀌거나 사라지지 않는다.
_Avoid_: 매핑, 레코드
