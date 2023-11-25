CREATE TABLE IF NOT EXISTS korean (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO korean (author, title, message)
VALUES
    ('김민준', '서울의 새로운 카페', '서울 중심부에 새로운 카페가 문을 열었습니다. 현대적인 디자인과 독특한 커피 선택이 특징입니다.'),
    ('이하은', '축구 경기 리뷰', '어제 열린 축구 경기에서 화려한 골이 터졌습니다. 마지막 순간의 반전이 경기의 하이라이트였습니다.'),
    ('박지후', '지역 축제 개최 소식', '이번 주말 지역 축제가 열립니다. 다양한 음식과 공연이 준비되어 있어 기대가 됩니다.');

CREATE INDEX idx_korean
ON korean
USING bm25 ((korean.*))
WITH (
    text_fields='{
        author: {tokenizer: {type: "lindera"}, record: "position"},
        title: {tokenizer: {type: "lindera"}, record: "position"},
        message: {tokenizer: {type: "lindera"}, record: "position"}
    }'
);

SELECT * FROM korean WHERE korean @@@ 'author:김민준';
SELECT * FROM korean WHERE korean @@@ 'title:"경기"';
SELECT * FROM korean WHERE korean @@@ 'message:"지역 축제"';
