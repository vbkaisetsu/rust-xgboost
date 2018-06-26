use std::{slice, ffi, ptr, path::Path};
use libc::{c_uint, c_float};
use std::os::unix::ffi::OsStrExt;

use xgboost_sys;

use super::{XGBResult, XGBError};

static KEY_ROOT_INDEX: &'static str = "root_index";
static KEY_LABEL: &'static str = "label";
static KEY_WEIGHT: &'static str = "weight";
static KEY_BASE_MARGIN: &'static str = "base_margin";

/// Data Matrix used in XGBoost.
pub struct DMatrix {
    pub(super) handle: xgboost_sys::DMatrixHandle,
    num_rows: usize,
    num_cols: usize,
}

impl DMatrix {
    fn new(handle: xgboost_sys::DMatrixHandle) -> XGBResult<Self> {
        let mut out = 0;
        xgb_call!(xgboost_sys::XGDMatrixNumRow(handle, &mut out))?;
        let num_rows = out as usize;

        let mut out = 0;
        xgb_call!(xgboost_sys::XGDMatrixNumCol(handle, &mut out))?;
        let num_cols = out as usize;

        info!("Loaded DMatrix with shape: {}x{}", num_rows, num_cols);
        Ok(DMatrix { handle, num_rows, num_cols })
    }

    /// Create a new `DMatrix` from given file (LibSVM or binary format).
    pub fn load<P: AsRef<Path>>(path: P) -> XGBResult<Self> {
        debug!("Loading DMatrix from: {}", path.as_ref().display());
        let mut handle = ptr::null_mut();
        let fname = ffi::CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
        let silent = true;
        xgb_call!(xgboost_sys::XGDMatrixCreateFromFile(fname.as_ptr(), silent as i32, &mut handle))?;
        Ok(DMatrix::new(handle)?)
    }

    /// Create a new `DMatrix` from a sparse CSR matrix.
    ///
    /// If `num_cols` is set to None, number of columns will be inferred from given data.
    pub fn from_csr(indptr: &[usize], indices: &[u32], data: &[f32], num_cols: Option<usize>) -> XGBResult<Self> {
        assert_eq!(indices.len(), data.len());
        let mut handle = ptr::null_mut();
        let num_cols = num_cols.unwrap_or(0); // guess from data if 0
        xgb_call!(xgboost_sys::XGDMatrixCreateFromCSREx(indptr.as_ptr(),
                                                        indices.as_ptr(),
                                                        data.as_ptr(),
                                                        indptr.len(),
                                                        data.len(),
                                                        num_cols,
                                                        &mut handle))?;
        Ok(DMatrix::new(handle)?)
    }

    /// Create a new `DMatrix` from a sparse CSC matrix.
    ///
    /// If `num_rows` is set to None, number of columns will be inferred from given data.
    pub fn from_csc(indptr: &[usize], indices: &[u32], data: &[f32], num_rows: Option<usize>) -> XGBResult<Self> {
        assert_eq!(indices.len(), data.len());
        let mut handle = ptr::null_mut();
        let num_rows = num_rows.unwrap_or(0); // guess from data if 0
        xgb_call!(xgboost_sys::XGDMatrixCreateFromCSCEx(indptr.as_ptr(),
                                                        indices.as_ptr(),
                                                        data.as_ptr(),
                                                        indptr.len(),
                                                        data.len(),
                                                        num_rows,
                                                        &mut handle))?;
        Ok(DMatrix::new(handle)?)
    }

    // TODO: can this be simplified? Should it just take an ndarray matrix instead, with missing as a default?
    pub fn from_dense(data: &[f32], num_rows: usize, num_cols: usize, missing: f32) -> XGBResult<Self> {
        let mut handle = ptr::null_mut();
        xgb_call!(xgboost_sys::XGDMatrixCreateFromMat(data.as_ptr(),
                                                      num_rows as xgboost_sys::bst_ulong,
                                                      num_cols as xgboost_sys::bst_ulong,
                                                      missing,
                                                      &mut handle))?;
        Ok(DMatrix::new(handle)?)
    }

    /// Serialise this `DMatrix` as a binary file.
    pub fn save<P: AsRef<Path>>(&self, path: P, silent: bool) -> XGBResult<()> {
        debug!("Writing DMatrix to: {}", path.as_ref().display());
        let fname = ffi::CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
        xgb_call!(xgboost_sys::XGDMatrixSaveBinary(self.handle, fname.as_ptr(), silent as i32))
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn num_cols(&self) -> usize {
        self.num_cols
    }

    /// Gets the specified root index of each instance, can be used for multi task setting.
    pub fn get_root_index(&self) -> XGBResult<&[u32]> {
        self.get_uint_info(KEY_ROOT_INDEX)
    }

    /// Sets the specified root index of each instance, can be used for multi task setting.
    pub fn set_root_index(&mut self, array: &[u32]) -> XGBResult<()> {
        self.set_uint_info(KEY_ROOT_INDEX, array)
    }

    /// Get ground truth labels for each row of this matrix.
    pub fn get_labels(&self) -> XGBResult<&[f32]> {
        self.get_float_info(KEY_LABEL)
    }

    /// Set ground truth labels for each row of this matrix.
    pub fn set_labels(&mut self, array: &[f32]) -> XGBResult<()> {
        self.set_float_info(KEY_LABEL, array)
    }

    /// Get weights of each instance.
    pub fn get_weights(&self) -> XGBResult<&[f32]> {
        self.get_float_info(KEY_WEIGHT)
    }

    /// Set weights of each instance.
    pub fn set_weights(&mut self, array: &[f32]) -> XGBResult<()> {
        self.set_float_info(KEY_WEIGHT, array)
    }

    /// Get base margin.
    pub fn get_base_margin(&self) -> XGBResult<&[f32]> {
        self.get_float_info(KEY_BASE_MARGIN)
    }

    /// Set base margin.
    ///
    /// If specified, xgboost will start from this margin, can be used to specify initial prediction to boost from.
    pub fn set_base_margin(&mut self, array: &[f32]) -> XGBResult<()> {
        self.set_float_info(KEY_BASE_MARGIN, array)
    }

    /// Set the index for the beginning and end of a group.
    ///
    /// Needed when the learning task is ranking.
    pub fn set_group(&mut self, group: &[u32]) -> XGBResult<()> {
        xgb_call!(xgboost_sys::XGDMatrixSetGroup(self.handle, group.as_ptr(), group.len() as u64))
    }

    fn get_float_info(&self, field: &str) -> XGBResult<&[f32]> {
        let field = ffi::CString::new(field).unwrap();
        let mut out_len = 0;
        let mut out_dptr = ptr::null();
        xgb_call!(xgboost_sys::XGDMatrixGetFloatInfo(self.handle,
                                                     field.as_ptr(),
                                                     &mut out_len,
                                                     &mut out_dptr))?;

        Ok(unsafe { slice::from_raw_parts(out_dptr as *mut c_float, out_len as usize) })
    }

    fn set_float_info(&mut self, field: &str, array: &[f32]) -> XGBResult<()> {
        let field = ffi::CString::new(field).unwrap();
        xgb_call!(xgboost_sys::XGDMatrixSetFloatInfo(self.handle,
                                                     field.as_ptr(),
                                                     array.as_ptr(),
                                                     array.len() as u64))
    }

    fn get_uint_info(&self, field: &str) -> XGBResult<&[u32]> {
        let field = ffi::CString::new(field).unwrap();
        let mut out_len = 0;
        let mut out_dptr = ptr::null();
        xgb_call!(xgboost_sys::XGDMatrixGetUIntInfo(self.handle,
                                                    field.as_ptr(),
                                                    &mut out_len,
                                                    &mut out_dptr))?;

        Ok(unsafe { slice::from_raw_parts(out_dptr as *mut c_uint, out_len as usize) })
    }

    fn set_uint_info(&mut self, field: &str, array: &[u32]) -> XGBResult<()> {
        let field = ffi::CString::new(field).unwrap();
        xgb_call!(xgboost_sys::XGDMatrixSetUIntInfo(self.handle,
                                                    field.as_ptr(),
                                                    array.as_ptr(),
                                                    array.len() as u64))
    }
}

impl Drop for DMatrix {
    fn drop(&mut self) {
        xgb_call!(xgboost_sys::XGDMatrixFree(self.handle)).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use tempfile;
    use super::*;
    fn read_train_matrix() -> XGBResult<DMatrix> {
        DMatrix::load("xgboost-sys/xgboost/demo/data/agaricus.txt.train")
    }

    #[test]
    fn read_matrix() {
        assert!(read_train_matrix().is_ok());
    }

    #[test]
    fn read_num_rows() {
        assert_eq!(read_train_matrix().unwrap().num_rows(), 6513);
    }

    #[test]
    fn read_num_cols() {
        assert_eq!(read_train_matrix().unwrap().num_cols(), 127);
    }

    #[test]
    fn writing_and_reading() {
        let dmat = read_train_matrix().unwrap();

        let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let out_path = tmp_dir.path().join("dmat.bin");
        dmat.save(&out_path, true).unwrap();

        let dmat2 = DMatrix::load(&out_path).unwrap();

        assert_eq!(dmat.num_rows(), dmat2.num_rows());
        assert_eq!(dmat.num_cols(), dmat2.num_cols());
        // TODO: check contents as well, if possible
    }

    #[test]
    fn get_set_root_index() {
        let mut dmat = read_train_matrix().unwrap();
        assert_eq!(dmat.get_root_index().unwrap(), &[]);

        let root_index = [3, 22, 1];
        assert!(dmat.set_root_index(&root_index).is_ok());
        assert_eq!(dmat.get_root_index().unwrap(), &[3, 22, 1]);
    }

    #[test]
    fn get_set_labels() {
        let mut dmat = read_train_matrix().unwrap();
        assert_eq!(dmat.get_labels().unwrap().len(), 6513);

        let label = [0.1, 0.0 -4.5, 11.29842, 333333.33];
        assert!(dmat.set_labels(&label).is_ok());
        assert_eq!(dmat.get_labels().unwrap(), label);
    }

    #[test]
    fn get_set_weights() {
        let mut dmat = read_train_matrix().unwrap();
        assert_eq!(dmat.get_weights().unwrap(), &[]);

        let weight = [1.0, 10.0, -123.456789, 44.9555];
        assert!(dmat.set_weights(&weight).is_ok());
        assert_eq!(dmat.get_weights().unwrap(), weight);
    }

    #[test]
    fn get_set_base_margin() {
        let mut dmat = read_train_matrix().unwrap();
        assert_eq!(dmat.get_base_margin().unwrap(), &[]);

        let base_margin = [0.00001, 0.000002, 1.23];
        assert!(dmat.set_base_margin(&base_margin).is_ok());
        assert_eq!(dmat.get_base_margin().unwrap(), base_margin);
    }

    #[test]
    fn set_group() {
        let mut dmat = read_train_matrix().unwrap();

        let group = [1, 2, 3];
        assert!(dmat.set_group(&group).is_ok());
    }

    #[test]
    fn from_csr() {
        let indptr = [0, 2, 3, 6, 8];
        let indices = [0, 2, 2, 0, 1, 2, 1, 2];
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        let dmat = DMatrix::from_csr(&indptr, &indices, &data, None).unwrap();
        assert_eq!(dmat.num_rows(), 4);
        assert_eq!(dmat.num_cols(), 3);

        let dmat = DMatrix::from_csr(&indptr, &indices, &data, Some(10)).unwrap();
        assert_eq!(dmat.num_rows(), 4);
        assert_eq!(dmat.num_cols(), 10);
    }

    #[test]
    fn from_csc() {
        let indptr = [0, 2, 3, 6, 8];
        let indices = [0, 2, 2, 0, 1, 2, 1, 2];
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        let dmat = DMatrix::from_csc(&indptr, &indices, &data, None).unwrap();
        assert_eq!(dmat.num_rows(), 3);
        assert_eq!(dmat.num_cols(), 4);

        let dmat = DMatrix::from_csc(&indptr, &indices, &data, Some(10)).unwrap();
        assert_eq!(dmat.num_rows(), 10);
        assert_eq!(dmat.num_cols(), 4);
    }

    #[test]
    fn from_dense() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let dmat = DMatrix::from_dense(&data, 2, 3, 0.0).unwrap();
        assert_eq!(dmat.num_rows(), 2);
        assert_eq!(dmat.num_cols(), 3);

        let dmat = DMatrix::from_dense(&data, 1, 6, 0.0).unwrap();
        assert_eq!(dmat.num_rows(), 1);
        assert_eq!(dmat.num_cols(), 6);

        let dmat = DMatrix::from_dense(&data, 10, 20, 0.5).unwrap();
        assert_eq!(dmat.num_rows(), 10);
        assert_eq!(dmat.num_cols(), 20);
    }
}
