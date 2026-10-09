use optic_bridge_core::{GitObjectId, WorkspacePath};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionBlob {
    pub path: WorkspacePath,
    pub object: GitObjectId,
    pub executable: bool,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionBlobBatch {
    pub head: GitObjectId,
    pub blobs: Vec<SessionBlob>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExpectedGitBlob {
    pub(crate) path: WorkspacePath,
    pub(crate) object: GitObjectId,
    pub(crate) size: u64,
    pub(crate) executable: bool,
}

pub(crate) fn expected_batch_output_bytes(
    expected: &[ExpectedGitBlob],
) -> Result<u64, GitBlobBatchError> {
    let mut total = 0_u64;
    for blob in expected {
        let object_len = u64::try_from(blob.object.as_str().len())
            .map_err(|_| GitBlobBatchError::SizeOverflow)?;
        let size_digits = u64::try_from(blob.size.to_string().len())
            .map_err(|_| GitBlobBatchError::SizeOverflow)?;
        // "<oid> blob <size>\n" + raw blob bytes + trailing protocol newline.
        let framing = object_len
            .checked_add(1 + 4 + 1)
            .and_then(|value| value.checked_add(size_digits))
            .and_then(|value| value.checked_add(1))
            .and_then(|value| value.checked_add(blob.size))
            .and_then(|value| value.checked_add(1))
            .ok_or(GitBlobBatchError::SizeOverflow)?;
        total = total
            .checked_add(framing)
            .ok_or(GitBlobBatchError::SizeOverflow)?;
    }
    Ok(total)
}

pub(crate) fn parse_cat_file_batch(
    expected: &[ExpectedGitBlob],
    bytes: &[u8],
) -> Result<Vec<SessionBlob>, GitBlobBatchError> {
    let mut cursor = 0_usize;
    let mut blobs = Vec::with_capacity(expected.len());

    for item in expected {
        let header_end = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| cursor + offset)
            .ok_or(GitBlobBatchError::Malformed)?;
        let header = std::str::from_utf8(&bytes[cursor..header_end])
            .map_err(|_| GitBlobBatchError::Malformed)?;
        let fields = header.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() != 3
            || fields[0] != item.object.as_str()
            || fields[1] != "blob"
            || fields[2]
                .parse::<u64>()
                .map_err(|_| GitBlobBatchError::Malformed)?
                != item.size
        {
            return Err(GitBlobBatchError::ObjectMismatch);
        }

        cursor = header_end
            .checked_add(1)
            .ok_or(GitBlobBatchError::SizeOverflow)?;
        let blob_len = usize::try_from(item.size).map_err(|_| GitBlobBatchError::SizeOverflow)?;
        let blob_end = cursor
            .checked_add(blob_len)
            .ok_or(GitBlobBatchError::SizeOverflow)?;
        if blob_end >= bytes.len() || bytes[blob_end] != b'\n' {
            return Err(GitBlobBatchError::Malformed);
        }
        blobs.push(SessionBlob {
            path: item.path.clone(),
            object: item.object.clone(),
            executable: item.executable,
            bytes: bytes[cursor..blob_end].to_vec(),
        });
        cursor = blob_end + 1;
    }

    if cursor != bytes.len() {
        return Err(GitBlobBatchError::TrailingData);
    }
    Ok(blobs)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GitBlobBatchError {
    #[error("Git blob batch framing is malformed")]
    Malformed,
    #[error("Git blob batch object/type/size does not match the server-derived manifest")]
    ObjectMismatch,
    #[error("Git blob batch contains trailing data")]
    TrailingData,
    #[error("Git blob batch size arithmetic overflowed")]
    SizeOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: char) -> GitObjectId {
        GitObjectId::parse(std::iter::repeat_n(byte, 40).collect::<String>()).expect("oid")
    }

    #[test]
    fn batch_parser_preserves_binary_content_and_order() {
        let expected = vec![
            ExpectedGitBlob {
                path: WorkspacePath::parse("a.bin").expect("path"),
                object: oid('1'),
                size: 4,
                executable: false,
            },
            ExpectedGitBlob {
                path: WorkspacePath::parse("bin/run").expect("path"),
                object: oid('2'),
                size: 2,
                executable: true,
            },
        ];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(format!("{} blob 4\n", expected[0].object.as_str()).as_bytes());
        bytes.extend_from_slice(&[0, b'\n', 0xff, b'x']);
        bytes.push(b'\n');
        bytes.extend_from_slice(format!("{} blob 2\n", expected[1].object.as_str()).as_bytes());
        bytes.extend_from_slice(b"ok\n");

        assert_eq!(
            u64::try_from(bytes.len()).expect("len"),
            expected_batch_output_bytes(&expected).expect("framing")
        );
        let parsed = parse_cat_file_batch(&expected, &bytes).expect("batch");
        assert_eq!(parsed[0].bytes, vec![0, b'\n', 0xff, b'x']);
        assert!(!parsed[0].executable);
        assert_eq!(parsed[1].bytes, b"ok");
        assert!(parsed[1].executable);
    }

    #[test]
    fn batch_parser_rejects_trailing_or_mismatched_output() {
        let expected = vec![ExpectedGitBlob {
            path: WorkspacePath::parse("a.txt").expect("path"),
            object: oid('a'),
            size: 1,
            executable: false,
        }];
        let wrong = format!("{} blob 2\nx\n", expected[0].object.as_str());
        assert_eq!(
            parse_cat_file_batch(&expected, wrong.as_bytes()).expect_err("size mismatch"),
            GitBlobBatchError::ObjectMismatch
        );

        let trailing = format!("{} blob 1\nx\nextra", expected[0].object.as_str());
        assert_eq!(
            parse_cat_file_batch(&expected, trailing.as_bytes()).expect_err("trailing"),
            GitBlobBatchError::TrailingData
        );
    }
}
