#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>

#include <algorithm>
#include <cmath>

extern "C" {

struct DocsyOpenCvMatch {
    double confidence;
    double match_ratio;
    double inlier_ratio;
    double shift_x;
    double shift_y;
};

int docsy_opencv_compare_luma(const unsigned char* previous,
                              const unsigned char* current,
                              int width,
                              int height,
                              DocsyOpenCvMatch* output) noexcept {
    if (!previous || !current || !output || width < 16 || height < 16) {
        return 0;
    }
    try {
        const cv::Mat previous_image(height, width, CV_8UC1,
                                     const_cast<unsigned char*>(previous));
        const cv::Mat current_image(height, width, CV_8UC1,
                                    const_cast<unsigned char*>(current));
        cv::Mat previous_x;
        cv::Mat previous_y;
        cv::Mat current_x;
        cv::Mat current_y;
        cv::Mat previous_edges;
        cv::Mat current_edges;
        cv::Sobel(previous_image, previous_x, CV_32F, 1, 0, 3);
        cv::Sobel(previous_image, previous_y, CV_32F, 0, 1, 3);
        cv::Sobel(current_image, current_x, CV_32F, 1, 0, 3);
        cv::Sobel(current_image, current_y, CV_32F, 0, 1, 3);
        cv::magnitude(previous_x, previous_y, previous_edges);
        cv::magnitude(current_x, current_y, current_edges);

        double response = 0.0;
        const cv::Point2d shift =
            cv::phaseCorrelate(previous_edges, current_edges, cv::noArray(), &response);
        if (!std::isfinite(response) || !std::isfinite(shift.x) || !std::isfinite(shift.y)) {
            return 0;
        }
        response = std::clamp(response, 0.0, 1.0);
        output->confidence = response;
        output->match_ratio = response;
        output->inlier_ratio = response;
        output->shift_x = shift.x;
        output->shift_y = shift.y;
        return 1;
    } catch (...) {
        return 0;
    }
}

}
